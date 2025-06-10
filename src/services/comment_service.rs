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
}

impl CommentService {
    pub fn new(repository: Arc<dyn CommentRepositoryTrait>) -> Self {
        Self { repository }
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
        let requires_moderation = self.requires_moderation(&request.content, &request.author_email);

        // Determine initial status
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
        let spam_keywords = [
            "viagra",
            "casino",
            "lottery",
            "winner",
            "congratulations",
            "click here",
            "free money",
            "make money fast",
            "work from home",
            "bitcoin investment",
            "crypto trading",
            "earn $",
            "guaranteed profit",
            "no risk",
            "limited time",
            "act now",
            "special offer",
            "discount",
            "buy now",
            "credit repair",
        ];

        let content_lower = content.to_lowercase();

        // Check for spam keywords
        let keyword_matches = spam_keywords
            .iter()
            .filter(|&keyword| content_lower.contains(keyword))
            .count();

        // If multiple spam keywords found, definitely spam
        if keyword_matches >= 2 {
            return true;
        }

        // Check for excessive links
        let link_count = content_lower.matches("http").count();
        if link_count > 2 {
            return true;
        }

        // Check for excessive capital letters
        let caps_count = content.chars().filter(|c| c.is_uppercase()).count();
        let total_letters = content.chars().filter(|c| c.is_alphabetic()).count();

        if total_letters > 0 && caps_count as f32 / total_letters as f32 > 0.5 {
            return true;
        }

        // Check for repeated characters (potential spam)
        let mut consecutive_count = 1;
        let mut prev_char = '\0';
        for ch in content.chars() {
            if ch == prev_char {
                consecutive_count += 1;
                if consecutive_count > 4 {
                    return true;
                }
            } else {
                consecutive_count = 1;
            }
            prev_char = ch;
        }

        // Check for excessive punctuation
        let punct_count = content.chars().filter(|c| c.is_ascii_punctuation()).count();
        if total_letters > 0 && punct_count as f32 / total_letters as f32 > 0.3 {
            return true;
        }

        false
    }

    fn requires_moderation(&self, content: &str, email: &str) -> bool {
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
                return true;
            }
        }

        // Very long comments require moderation
        if content.len() > 2000 {
            return true;
        }

        // Short comments from trusted domains can be auto-approved
        if is_trusted_domain && content.len() > 10 && content.len() < 500 {
            return false;
        }

        // First-time commenters from non-trusted domains require moderation
        true
    }

    async fn check_rate_limit(&self, ip_address: &str) -> Result<bool> {
        // Enhanced rate limiting: check comments from this IP in the last hour
        let recent_comments_count = self
            .repository
            .count_recent_comments_by_ip(ip_address, 3600)
            .await?;

        // Allow max 3 comments per hour from same IP
        if recent_comments_count >= 3 {
            return Ok(true);
        }

        // Check comments from this IP in the last 5 minutes for rapid spam
        let very_recent_comments = self
            .repository
            .count_recent_comments_by_ip(ip_address, 300)
            .await?;

        // Allow max 1 comment per 5 minutes from same IP
        if very_recent_comments >= 1 {
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
