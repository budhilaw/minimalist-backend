use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use anyhow::{Context, Result};

use crate::models::comment::{
    Comment, CommentModerationInfo, CommentQuery, CommentResponse, CommentStats, CommentsResponse,
    CreateCommentRequest, UpdateCommentStatusRequest,
};
use crate::utils::errors::AppError;

#[async_trait]
pub trait CommentRepositoryTrait: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Comment>, AppError>;
    async fn find_all(&self, query: CommentQuery) -> Result<CommentsResponse, AppError>;
    async fn create(
        &self,
        comment: CreateCommentRequest,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Result<Comment, AppError>;
    async fn create_with_status(
        &self,
        comment: CreateCommentRequest,
        ip_address: Option<String>,
        user_agent: Option<String>,
        status: String,
    ) -> Result<Comment, AppError>;
    async fn update_status(
        &self,
        id: Uuid,
        status: UpdateCommentStatusRequest,
    ) -> Result<Comment, AppError>;
    async fn delete(&self, id: Uuid) -> Result<(), AppError>;
    async fn get_by_post(
        &self,
        post_id: Uuid,
        include_replies: bool,
    ) -> Result<Vec<Comment>, AppError>;
    async fn get_pending_moderation(&self) -> Result<Vec<CommentModerationInfo>, AppError>;
    async fn get_stats(&self) -> Result<CommentStats, AppError>;
    async fn get_replies(&self, parent_id: Uuid) -> Result<Vec<Comment>, AppError>;
    async fn bulk_update_status(&self, ids: Vec<Uuid>, status: String) -> Result<i64, AppError>;
    async fn count_recent_comments_by_ip(
        &self,
        ip_address: &str,
        seconds_ago: i64,
    ) -> Result<i64, AppError>;
}

pub struct CommentRepository {
    pool: PgPool,
}

impl CommentRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl CommentRepositoryTrait for CommentRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Comment>, AppError> {
        let comment = sqlx::query_as::<_, Comment>(
            r#"
            SELECT id, post_id, author_name, author_email, content, status, 
                   ip_address::text as ip_address, user_agent, parent_id, created_at, updated_at
            FROM comments 
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch comment by id")?;

        Ok(comment)
    }

    async fn find_all(&self, query: CommentQuery) -> Result<CommentsResponse, AppError> {
        let limit = query.limit.unwrap_or(20).min(100);
        let offset = (query.page.unwrap_or(1) - 1) * limit;

        // Get total count
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM comments")
            .fetch_one(&self.pool)
            .await
            .context("Failed to count comments")?;

        // Get comments with simplified query
        let comments = sqlx::query_as::<_, Comment>(
            r#"
            SELECT id, post_id, author_name, author_email, content, status, 
                   ip_address::text as ip_address, user_agent, parent_id, created_at, updated_at
            FROM comments 
            ORDER BY created_at DESC 
            LIMIT $1 OFFSET $2
            "#,
        )
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch comments")?;

        let total_pages = (total as f64 / limit as f64).ceil() as u32;

        // Convert comments to response format
        let comment_responses: Vec<CommentResponse> =
            comments.into_iter().map(CommentResponse::from).collect();

        Ok(CommentsResponse {
            comments: comment_responses,
            total,
            page: query.page.unwrap_or(1),
            limit,
            total_pages,
        })
    }

    async fn create(
        &self,
        comment: CreateCommentRequest,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Result<Comment, AppError> {
        let created_comment = sqlx::query_as::<_, Comment>(
            r#"
            INSERT INTO comments (
                post_id, author_name, author_email, content, status, 
                ip_address, user_agent, parent_id
            )
            VALUES ($1, $2, $3, $4, 'pending', $5::inet, $6, $7)
            RETURNING id, post_id, author_name, author_email, content, status, 
                      ip_address::text as ip_address, user_agent, parent_id, created_at, updated_at
            "#,
        )
        .bind(comment.post_id)
        .bind(&comment.author_name)
        .bind(&comment.author_email)
        .bind(&comment.content)
        .bind(ip_address)
        .bind(user_agent)
        .bind(comment.parent_id)
        .fetch_one(&self.pool)
        .await
        .context("Failed to create comment")?;

        Ok(created_comment)
    }

    async fn create_with_status(
        &self,
        comment: CreateCommentRequest,
        ip_address: Option<String>,
        user_agent: Option<String>,
        status: String,
    ) -> Result<Comment, AppError> {
        let created_comment = sqlx::query_as::<_, Comment>(
            r#"
            INSERT INTO comments (
                post_id, author_name, author_email, content, status, 
                ip_address, user_agent, parent_id
            )
            VALUES ($1, $2, $3, $4, $5, $6::inet, $7, $8)
            RETURNING id, post_id, author_name, author_email, content, status, 
                      ip_address::text as ip_address, user_agent, parent_id, created_at, updated_at
            "#,
        )
        .bind(comment.post_id)
        .bind(&comment.author_name)
        .bind(&comment.author_email)
        .bind(&comment.content)
        .bind(&status)
        .bind(ip_address)
        .bind(user_agent)
        .bind(comment.parent_id)
        .fetch_one(&self.pool)
        .await
        .context("Failed to create comment with status")?;

        Ok(created_comment)
    }

    async fn update_status(
        &self,
        id: Uuid,
        status: UpdateCommentStatusRequest,
    ) -> Result<Comment, AppError> {
        let updated_comment = sqlx::query_as::<_, Comment>(
            r#"
            UPDATE comments 
            SET status = $1, updated_at = NOW()
            WHERE id = $2
            RETURNING id, post_id, author_name, author_email, content, status, 
                      ip_address::text as ip_address, user_agent, parent_id, created_at, updated_at
            "#,
        )
        .bind(&status.status)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to update comment status")?
        .ok_or(AppError::NotFound("Comment not found".to_string()))?;

        Ok(updated_comment)
    }

    async fn delete(&self, id: Uuid) -> Result<(), AppError> {
        let result = sqlx::query("DELETE FROM comments WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .context("Failed to delete comment")?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Comment not found".to_string()));
        }

        Ok(())
    }

    async fn get_by_post(
        &self,
        post_id: Uuid,
        include_replies: bool,
    ) -> Result<Vec<Comment>, AppError> {
        let comments = if include_replies {
            sqlx::query_as::<_, Comment>(
                r#"
                SELECT id, post_id, author_name, author_email, content, status, 
                       ip_address::text as ip_address, user_agent, parent_id, created_at, updated_at
                FROM comments 
                WHERE post_id = $1 AND status = 'approved'
                ORDER BY created_at ASC
                "#,
            )
        } else {
            sqlx::query_as::<_, Comment>(
                r#"
                SELECT id, post_id, author_name, author_email, content, status, 
                       ip_address::text as ip_address, user_agent, parent_id, created_at, updated_at
                FROM comments 
                WHERE post_id = $1 AND status = 'approved' AND parent_id IS NULL
                ORDER BY created_at ASC
                "#,
            )
        }
        .bind(post_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch comments by post")?;

        Ok(comments)
    }

    async fn get_pending_moderation(&self) -> Result<Vec<CommentModerationInfo>, AppError> {
        let comments = sqlx::query_as::<_, CommentModerationInfo>(
            r#"
            SELECT 
                c.id, c.post_id, p.title as post_title, c.author_name, 
                c.author_email, c.content, c.status, c.ip_address::text as ip_address, 
                c.user_agent, c.created_at
            FROM comments c
            LEFT JOIN posts p ON c.post_id = p.id
            WHERE c.status = 'pending'
            ORDER BY c.created_at ASC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch pending comments")?;

        Ok(comments)
    }

    async fn get_stats(&self) -> Result<CommentStats, AppError> {
        let total_comments: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM comments")
            .fetch_one(&self.pool)
            .await
            .context("Failed to count total comments")?;

        let pending_comments: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM comments WHERE status = 'pending'")
                .fetch_one(&self.pool)
                .await
                .context("Failed to count pending comments")?;

        let approved_comments: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM comments WHERE status = 'approved'")
                .fetch_one(&self.pool)
                .await
                .context("Failed to count approved comments")?;

        let rejected_comments: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM comments WHERE status = 'rejected'")
                .fetch_one(&self.pool)
                .await
                .context("Failed to count rejected comments")?;

        let comments_this_month: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM comments WHERE EXTRACT(MONTH FROM created_at) = EXTRACT(MONTH FROM CURRENT_DATE) AND EXTRACT(YEAR FROM created_at) = EXTRACT(YEAR FROM CURRENT_DATE)"
        )
        .fetch_one(&self.pool)
        .await
        .context("Failed to count comments this month")?;

        Ok(CommentStats {
            total_comments,
            pending_comments,
            approved_comments,
            rejected_comments,
            comments_this_month,
        })
    }

    async fn get_replies(&self, parent_id: Uuid) -> Result<Vec<Comment>, AppError> {
        let replies = sqlx::query_as::<_, Comment>(
            r#"
            SELECT id, post_id, author_name, author_email, content, status, 
                   ip_address::text as ip_address, user_agent, parent_id, created_at, updated_at
            FROM comments 
            WHERE parent_id = $1 AND status = 'approved'
            ORDER BY created_at ASC
            "#,
        )
        .bind(parent_id)
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch comment replies")?;

        Ok(replies)
    }

    async fn bulk_update_status(&self, ids: Vec<Uuid>, status: String) -> Result<i64, AppError> {
        let result =
            sqlx::query("UPDATE comments SET status = $1, updated_at = NOW() WHERE id = ANY($2)")
                .bind(&status)
                .bind(&ids)
                .execute(&self.pool)
                .await
                .context("Failed to bulk update comment status")?;

        Ok(result.rows_affected() as i64)
    }

    async fn count_recent_comments_by_ip(
        &self,
        ip_address: &str,
        seconds_ago: i64,
    ) -> Result<i64, AppError> {
        let result = sqlx::query_scalar(
            "SELECT COUNT(*) FROM comments WHERE ip_address = $1::inet AND created_at >= NOW() - INTERVAL '1 second' * $2"
        )
        .bind(ip_address)
        .bind(seconds_ago)
        .fetch_one(&self.pool)
        .await
        .context("Failed to count recent comments by IP")?;

        Ok(result)
    }
}
