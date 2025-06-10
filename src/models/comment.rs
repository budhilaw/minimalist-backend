use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Comment {
    pub id: Uuid,
    pub post_id: Uuid,
    pub author_name: String,
    pub author_email: String,
    pub content: String,
    pub status: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub parent_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct CommentResponse {
    pub id: Uuid,
    pub post_id: Uuid,
    pub author_name: String,
    pub author_email: String,
    pub content: String,
    pub status: String,
    pub parent_id: Option<Uuid>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub replies: Option<Vec<CommentResponse>>,
}

impl From<Comment> for CommentResponse {
    fn from(comment: Comment) -> Self {
        Self {
            id: comment.id,
            post_id: comment.post_id,
            author_name: comment.author_name,
            author_email: comment.author_email,
            content: comment.content,
            status: comment.status,
            parent_id: comment.parent_id,
            created_at: comment.created_at,
            updated_at: comment.updated_at,
            replies: None, // Will be populated separately if needed
        }
    }
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateCommentRequest {
    pub post_id: Uuid,
    #[validate(length(
        min = 1,
        max = 255,
        message = "Author name is required and must be less than 255 characters"
    ))]
    pub author_name: String,
    #[validate(email(message = "Please provide a valid email address"))]
    pub author_email: String,
    #[validate(length(
        min = 1,
        max = 2000,
        message = "Content is required and must be less than 2000 characters"
    ))]
    pub content: String,
    pub parent_id: Option<Uuid>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateCommentStatusRequest {
    #[validate(length(min = 1, message = "Status is required"))]
    pub status: String, // pending, approved, rejected
}

#[derive(Debug, Deserialize)]
pub struct CommentQuery {
    pub page: Option<u32>,
    pub limit: Option<u32>,
    pub post_id: Option<Uuid>,
    pub status: Option<String>,
    pub author_email: Option<String>,
    pub include_replies: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct CommentsResponse {
    pub comments: Vec<CommentResponse>,
    pub total: i64,
    pub page: u32,
    pub limit: u32,
    pub total_pages: u32,
}

#[derive(Debug, Serialize)]
pub struct CommentStats {
    pub total_comments: i64,
    pub pending_comments: i64,
    pub approved_comments: i64,
    pub rejected_comments: i64,
    pub comments_this_month: i64,
}

#[derive(Debug, Serialize, FromRow)]
pub struct CommentModerationInfo {
    pub id: Uuid,
    pub post_id: Uuid,
    pub post_title: String,
    pub author_name: String,
    pub author_email: String,
    pub content: String,
    pub status: String,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub created_at: DateTime<Utc>,
}
