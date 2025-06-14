use crate::utils::errors::AppError;
use std::sync::Arc;
use uuid::Uuid;
type Result<T> = std::result::Result<T, AppError>;

use crate::{
    models::comment::{
        Comment, CommentModerationInfo, CommentQuery, CommentStats, CommentsResponse,
        CreateCommentRequest, UpdateCommentStatusRequest,
    },
    repositories::comment_repository::CommentRepositoryTrait,
    services::admin_settings_service::AdminSettingsServiceTrait,
};

#[async_trait::async_trait]
pub trait CommentServiceTrait: Send + Sync {
    async fn get_all_comments(&self, query: CommentQuery) -> Result<CommentsResponse>;
    async fn get_comment_by_id(&self, id: Uuid) -> Result<Option<Comment>>;
    async fn create_comment(
        &self,
        request: CreateCommentRequest,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Result<Comment>;
    async fn update_comment_status(
        &self,
        id: Uuid,
        request: UpdateCommentStatusRequest,
    ) -> Result<Comment>;
    async fn delete_comment(&self, id: Uuid) -> Result<()>;
    async fn get_comments_by_post(
        &self,
        post_id: Uuid,
        include_replies: bool,
    ) -> Result<Vec<Comment>>;
    async fn get_comment_replies(&self, parent_id: Uuid) -> Result<Vec<Comment>>;
    async fn get_pending_comments(&self) -> Result<Vec<CommentModerationInfo>>;
    async fn get_comment_statistics(&self) -> Result<CommentStats>;
    async fn bulk_moderate_comments(&self, ids: Vec<Uuid>, status: String) -> Result<i64>;
    async fn approve_comment(&self, id: Uuid) -> Result<()>;
    async fn reject_comment(&self, id: Uuid) -> Result<()>;
}

#[derive(Clone)]
pub struct CommentService {
    repository: Arc<dyn CommentRepositoryTrait>,
    admin_settings_service: Arc<dyn AdminSettingsServiceTrait>,
}

impl CommentService {
    pub fn new(
        repository: Arc<dyn CommentRepositoryTrait>,
        admin_settings_service: Arc<dyn AdminSettingsServiceTrait>,
    ) -> Self {
        Self { 
            repository,
            admin_settings_service,
        }
    }

    // Check if comments are enabled in admin settings
    async fn check_comments_enabled(&self) -> Result<()> {
        let comments_enabled = self.admin_settings_service
            .is_feature_enabled("comments")
            .await
            .unwrap_or(true); // Default to enabled if check fails

        if !comments_enabled {
            return Err(AppError::Validation(
                "Comments are currently disabled".to_string(),
            ));
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl CommentServiceTrait for CommentService {
    async fn get_all_comments(&self, query: CommentQuery) -> Result<CommentsResponse> {
        // Business logic: Apply default pagination
        let query = CommentQuery {
            page: query.page.or(Some(1)),
            limit: query.limit.or(Some(20)),
            ..query
        };

        self.repository.find_all(query).await
    }

    async fn get_comment_by_id(&self, id: Uuid) -> Result<Option<Comment>> {
        self.repository.find_by_id(id).await
    }

    async fn create_comment(
        &self,
        request: CreateCommentRequest,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Result<Comment> {
        // Check if comments are enabled
        self.check_comments_enabled().await?;

        // Business logic: Validate comment content
        self.validate_comment_content(
            &request.content,
            &request.author_name,
            &request.author_email,
        )?;

        // Business logic: Check for spam
        if self.is_spam_content(&request.content) {
            return Err(AppError::Validation(
                "Comment appears to be spam and has been rejected".to_string(),
            ));
        }

        // Business logic: Rate limiting check by IP
        if let Some(ref ip) = ip_address {
            if self.check_rate_limit(ip).await? {
                return Err(AppError::Validation(
                    "Too many comments from this IP address. Please wait before posting again."
                        .to_string(),
                ));
            }
        }

        // Business logic: Auto-moderate based on content and email
        let requires_moderation = self.requires_moderation(&request.content, &request.author_email).await?;

        // Determine initial status based on admin settings and content analysis
        let initial_status = if requires_moderation {
            "pending"
        } else {
            "approved"
        };

        self.repository
            .create_with_status(request, ip_address, user_agent, initial_status.to_string())
            .await
    }

    async fn update_comment_status(
        &self,
        id: Uuid,
        request: UpdateCommentStatusRequest,
    ) -> Result<Comment> {
        // Business logic: Ensure comment exists
        if self.repository.find_by_id(id).await?.is_none() {
            return Err(AppError::NotFound("Comment not found".to_string()));
        }

        // Business logic: Validate status transition
        self.validate_status_transition(&request.status)?;

        self.repository.update_status(id, request).await
    }

    async fn delete_comment(&self, id: Uuid) -> Result<()> {
        // Business logic: Ensure comment exists
        let _comment = self
            .repository
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound("Comment not found".to_string()))?;

        // Business logic: Check if comment has replies
        let replies = self.repository.get_replies(id).await?;
        if !replies.is_empty() {
            return Err(AppError::Validation(
                "Cannot delete comment with replies. Delete replies first.".to_string(),
            ));
        }

        self.repository.delete(id).await
    }

    async fn get_comments_by_post(
        &self,
        post_id: Uuid,
        include_replies: bool,
    ) -> Result<Vec<Comment>> {
        // Check if comments are enabled for public viewing
        self.check_comments_enabled().await?;

        // Business logic: Only return approved comments for public viewing
        let mut comments = self
            .repository
            .get_by_post(post_id, include_replies)
            .await?;

        // Filter out non-approved comments (business rule for public API)
        comments.retain(|comment| comment.status == "approved");

        Ok(comments)
    }

    async fn get_comment_replies(&self, parent_id: Uuid) -> Result<Vec<Comment>> {
        // Check if comments are enabled
        self.check_comments_enabled().await?;

        // Business logic: Ensure parent comment exists
        if self.repository.find_by_id(parent_id).await?.is_none() {
            return Err(AppError::NotFound("Parent comment not found".to_string()));
        }

        let mut replies = self.repository.get_replies(parent_id).await?;

        // Filter out non-approved replies for public viewing
        replies.retain(|reply| reply.status == "approved");

        Ok(replies)
    }

    async fn get_pending_comments(&self) -> Result<Vec<CommentModerationInfo>> {
        self.repository.get_pending_moderation().await
    }

    async fn get_comment_statistics(&self) -> Result<CommentStats> {
        self.repository.get_stats().await
    }

    async fn bulk_moderate_comments(&self, ids: Vec<Uuid>, status: String) -> Result<i64> {
        // Business logic: Validate bulk operation
        if ids.is_empty() {
            return Err(AppError::Validation("No comment IDs provided".to_string()));
        }

        if ids.len() > 100 {
            return Err(AppError::Validation(
                "Cannot bulk moderate more than 100 comments at once".to_string(),
            ));
        }

        // Business logic: Validate status
        self.validate_status_transition(&status)?;

        self.repository.bulk_update_status(ids, status).await
    }

    async fn approve_comment(&self, id: Uuid) -> Result<()> {
        let request = UpdateCommentStatusRequest {
            status: "approved".to_string(),
        };

        self.update_comment_status(id, request).await?;
        Ok(())
    }

    async fn reject_comment(&self, id: Uuid) -> Result<()> {
        let request = UpdateCommentStatusRequest {
            status: "rejected".to_string(),
        };

        self.update_comment_status(id, request).await?;
        Ok(())
    }
}

impl CommentService {
    fn validate_comment_content(
        &self,
        content: &str,
        author_name: &str,
        author_email: &str,
    ) -> Result<()> {
        if content.trim().is_empty() {
            return Err(AppError::Validation(
                "Comment content cannot be empty".to_string(),
            ));
        }

        if content.trim().len() < 5 {
            return Err(AppError::Validation(
                "Comment must be at least 5 characters long".to_string(),
            ));
        }

        if content.len() > 5000 {
            return Err(AppError::Validation(
                "Comment cannot exceed 5000 characters".to_string(),
            ));
        }

        if author_name.trim().is_empty() {
            return Err(AppError::Validation("Author name is required".to_string()));
        }

        if author_name.len() > 100 {
            return Err(AppError::Validation(
                "Author name cannot exceed 100 characters".to_string(),
            ));
        }

        if author_email.trim().is_empty() {
            return Err(AppError::Validation("Author email is required".to_string()));
        }

        if !author_email.contains('@') {
            return Err(AppError::Validation("Invalid email address".to_string()));
        }

        Ok(())
    }

    fn is_spam_content(&self, content: &str) -> bool {
        let content_lower = content.to_lowercase();

        // Common spam indicators
        let spam_keywords = [
            "viagra", "casino", "lottery", "winner", "congratulations",
            "click here", "free money", "make money fast", "work from home",
            "buy now", "limited time", "act now", "urgent", "guaranteed",
            "no risk", "100% free", "amazing deal", "incredible offer",
        ];

        for keyword in &spam_keywords {
            if content_lower.contains(keyword) {
                return true;
            }
        }

        // Check for excessive links
        let link_count = content.matches("http").count();
        if link_count > 2 {
            return true;
        }

        // Check for excessive capitalization
        let caps_count = content.chars().filter(|c| c.is_uppercase()).count();
        let total_letters = content.chars().filter(|c| c.is_alphabetic()).count();
        if total_letters > 0 && caps_count as f32 / total_letters as f32 > 0.5 {
            return true;
        }

        // Check for excessive punctuation
        let punct_count = content.chars().filter(|c| c.is_ascii_punctuation()).count();
        if total_letters > 0 && punct_count as f32 / total_letters as f32 > 0.3 {
            return true;
        }

        false
    }

    async fn requires_moderation(&self, content: &str, email: &str) -> Result<bool> {
        // Check admin setting first - if comment approval is required, all comments need moderation
        let settings = self.admin_settings_service.get_all_settings().await.unwrap_or_default();
        if settings.security.comment_approval_required {
            return Ok(true);
        }

        // Auto-approve comments from known good email domains (for trusted organizations)
        let trusted_domains = ["@gmail.com", "@outlook.com", "@yahoo.com", "@hotmail.com"];
        let is_trusted_domain = trusted_domains
            .iter()
            .any(|domain| email.to_lowercase().ends_with(domain));

        // Comments with certain keywords require moderation
        let moderation_keywords = [
            "admin",
            "moderator",
            "complaint",
            "report",
            "bug",
            "issue",
            "problem",
            "copyright",
            "dmca",
            "legal",
        ];
        let content_lower = content.to_lowercase();

        for keyword in &moderation_keywords {
            if content_lower.contains(keyword) {
                return Ok(true);
            }
        }

        // Very long comments require moderation
        if content.len() > 2000 {
            return Ok(true);
        }

        // Short comments from trusted domains can be auto-approved
        if is_trusted_domain && content.len() > 10 && content.len() < 500 {
            return Ok(false);
        }

        // First-time commenters from non-trusted domains require moderation
        Ok(true)
    }

    async fn check_rate_limit(&self, ip_address: &str) -> Result<bool> {
        // Get rate limiting settings from admin settings
        let settings = self.admin_settings_service.get_all_settings().await.unwrap_or_default();
        let rate_limit_settings = &settings.security.comment_rate_limit;

        // If rate limiting is disabled, allow all comments
        if !rate_limit_settings.enabled {
            return Ok(false);
        }

        // Check comments from this IP in the last hour
        let recent_comments_count = self
            .repository
            .count_recent_comments_by_ip(ip_address, 3600)
            .await?;

        // Check against configured hourly limit
        if recent_comments_count >= rate_limit_settings.max_comments_per_hour as i64 {
            return Ok(true);
        }

        // Check comments from this IP in the configured minute window
        let minute_window_seconds = rate_limit_settings.minute_window * 60;
        let very_recent_comments = self
            .repository
            .count_recent_comments_by_ip(ip_address, minute_window_seconds as i64)
            .await?;

        // Check against configured minute limit
        if very_recent_comments >= rate_limit_settings.max_comments_per_minute as i64 {
            return Ok(true);
        }

        Ok(false)
    }

    fn validate_status_transition(&self, status: &str) -> Result<()> {
        match status {
            "pending" | "approved" | "rejected" | "spam" => Ok(()),
            _ => Err(AppError::Validation(format!(
                "Invalid comment status: {}",
                status
            ))),
        }
    }
}
