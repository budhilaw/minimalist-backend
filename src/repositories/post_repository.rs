use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::post::{
    CreatePostRequest, Post, PostQuery, PostStats, PostsResponse, UpdatePostRequest,
};
use crate::utils::errors::AppError;

#[async_trait]
pub trait PostRepositoryTrait: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Post>, AppError>;
    async fn find_by_slug(&self, slug: &str) -> Result<Option<Post>, AppError>;
    async fn find_all(&self, query: PostQuery) -> Result<PostsResponse, AppError>;
    async fn create(&self, post: CreatePostRequest) -> Result<Post, AppError>;
    async fn update(&self, id: Uuid, post: UpdatePostRequest) -> Result<Post, AppError>;
    async fn delete(&self, id: Uuid) -> Result<(), AppError>;
    async fn get_published(&self, limit: Option<u32>) -> Result<Vec<Post>, AppError>;
    async fn get_featured(&self, limit: Option<u32>) -> Result<Vec<Post>, AppError>;
    async fn get_by_category(
        &self,
        category: &str,
        limit: Option<u32>,
    ) -> Result<Vec<Post>, AppError>;
    async fn get_by_tags(
        &self,
        tags: Vec<String>,
        limit: Option<u32>,
    ) -> Result<Vec<Post>, AppError>;
    async fn get_stats(&self) -> Result<PostStats, AppError>;
    async fn update_published_status(&self, id: Uuid, published: bool) -> Result<(), AppError>;
    async fn increment_view_count(&self, id: Uuid) -> Result<(), AppError>;
    async fn check_slug_exists(
        &self,
        slug: &str,
        exclude_id: Option<Uuid>,
    ) -> Result<bool, AppError>;
}

pub struct PostRepository {
    pool: PgPool,
}

impl PostRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PostRepositoryTrait for PostRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Post>, AppError> {
        let post = sqlx::query_as::<_, Post>(
            r#"
            SELECT id, title, slug, content, excerpt, category, tags, featured_image, featured, 
                   published, seo_title, seo_description, seo_keywords, view_count, 
                   published_at, created_at, updated_at
            FROM posts 
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch post by id")?;

        Ok(post)
    }

    async fn find_by_slug(&self, slug: &str) -> Result<Option<Post>, AppError> {
        let post = sqlx::query_as::<_, Post>(
            r#"
            SELECT id, title, slug, content, excerpt, category, tags, featured_image, featured, 
                   published, seo_title, seo_description, seo_keywords, view_count, 
                   published_at, created_at, updated_at
            FROM posts 
            WHERE slug = $1
            "#,
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch post by slug")?;

        Ok(post)
    }

    async fn find_all(&self, query: PostQuery) -> Result<PostsResponse, AppError> {
        let limit = query.limit.unwrap_or(10).min(100);
        let offset = (query.page.unwrap_or(1) - 1) * limit;

        // For simplicity, using basic query without complex dynamic binding
        let base_count_query = "SELECT COUNT(*) FROM posts";
        let base_posts_query = r#"
            SELECT id, title, slug, content, excerpt, category, tags, featured_image, featured, 
                   published, seo_title, seo_description, seo_keywords, view_count, 
                   published_at, created_at, updated_at
            FROM posts 
            ORDER BY created_at DESC 
            LIMIT $1 OFFSET $2
        "#;

        // Get total count
        let total: i64 = sqlx::query_scalar(base_count_query)
            .fetch_one(&self.pool)
            .await
            .context("Failed to count posts")?;

        // Get posts
        let posts = sqlx::query_as::<_, Post>(base_posts_query)
            .bind(limit as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await
            .context("Failed to fetch posts")?;

        let total_pages = (total as f64 / limit as f64).ceil() as u32;

        Ok(PostsResponse {
            posts: posts.into_iter().map(|p| p.into()).collect(),
            total,
            page: query.page.unwrap_or(1),
            limit,
            total_pages,
        })
    }

    async fn create(&self, post: CreatePostRequest) -> Result<Post, AppError> {
        let created_post = sqlx::query_as::<_, Post>(
            r#"
            INSERT INTO posts (
                title, slug, content, excerpt, category, tags, featured, 
                published, seo_title, seo_description, seo_keywords, published_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            RETURNING id, title, slug, content, excerpt, category, tags, featured, 
                      published, seo_title, seo_description, seo_keywords, view_count, 
                      published_at, created_at, updated_at
            "#,
        )
        .bind(&post.title)
        .bind(&post.slug)
        .bind(&post.content)
        .bind(&post.excerpt)
        .bind(&post.category)
        .bind(&post.tags)
        .bind(post.featured.unwrap_or(false))
        .bind(post.published.unwrap_or(false))
        .bind(&post.seo_title)
        .bind(&post.seo_description)
        .bind(&post.seo_keywords)
        .bind(if post.published.unwrap_or(false) {
            Some(chrono::Utc::now())
        } else {
            None
        })
        .fetch_one(&self.pool)
        .await
        .context("Failed to create post")?;

        Ok(created_post)
    }

    async fn update(&self, id: Uuid, post: UpdatePostRequest) -> Result<Post, AppError> {
        // Check if we're changing published status
        let current_published =
            sqlx::query_scalar::<_, bool>("SELECT published FROM posts WHERE id = $1")
                .bind(id)
                .fetch_optional(&self.pool)
                .await
                .context("Failed to check current published status")?
                .unwrap_or(false);

        let new_published = post.published.unwrap_or(false);
        let _published_at = if !current_published && new_published {
            Some(chrono::Utc::now())
        } else if current_published && !new_published {
            None
        } else {
            // Keep existing published_at, we'll use a sub-query
            None
        };

        let updated_post = sqlx::query_as::<_, Post>(
            r#"
            UPDATE posts 
            SET title = $1, slug = $2, content = $3, excerpt = $4, category = $5, 
                tags = $6, featured = $7, published = $8, seo_title = $9, 
                seo_description = $10, seo_keywords = $11, 
                published_at = CASE 
                    WHEN $8 = true AND published = false THEN NOW()
                    WHEN $8 = false THEN NULL
                    ELSE published_at
                END,
                updated_at = NOW()
            WHERE id = $12
            RETURNING id, title, slug, content, excerpt, category, tags, featured, 
                      published, seo_title, seo_description, seo_keywords, view_count, 
                      published_at, created_at, updated_at
            "#,
        )
        .bind(&post.title)
        .bind(&post.slug)
        .bind(&post.content)
        .bind(&post.excerpt)
        .bind(&post.category)
        .bind(&post.tags)
        .bind(post.featured.unwrap_or(false))
        .bind(new_published)
        .bind(&post.seo_title)
        .bind(&post.seo_description)
        .bind(&post.seo_keywords)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to update post")?
        .ok_or(AppError::NotFound("Post not found".to_string()))?;

        Ok(updated_post)
    }

    async fn delete(&self, id: Uuid) -> Result<(), AppError> {
        let result = sqlx::query("DELETE FROM posts WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .context("Failed to delete post")?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Post not found".to_string()));
        }

        Ok(())
    }

    async fn get_published(&self, limit: Option<u32>) -> Result<Vec<Post>, AppError> {
        use tracing::{info, error};
        
        let limit = limit.unwrap_or(10).min(50);
        
        info!("get_published: Starting with limit: {}", limit);

        let posts = sqlx::query_as::<_, Post>(
            r#"
            SELECT id, title, slug, content, excerpt, category, tags, featured_image, featured, 
                   published, seo_title, seo_description, seo_keywords, view_count, 
                   published_at, created_at, updated_at
            FROM posts 
            WHERE published = true 
            ORDER BY published_at DESC 
            LIMIT $1
            "#,
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| {
            error!("get_published: Database query failed: {:?}", e);
            AppError::from(anyhow::Error::from(e).context("Failed to fetch published posts"))
        })?;

        info!("get_published: Successfully fetched {} posts", posts.len());
        Ok(posts)
    }

    async fn get_featured(&self, limit: Option<u32>) -> Result<Vec<Post>, AppError> {
        let limit = limit.unwrap_or(5).min(20);

        let posts = sqlx::query_as::<_, Post>(
            r#"
            SELECT id, title, slug, content, excerpt, category, tags, featured_image, featured, 
                   published, seo_title, seo_description, seo_keywords, view_count, 
                   published_at, created_at, updated_at
            FROM posts 
            WHERE featured = true AND published = true
            ORDER BY published_at DESC 
            LIMIT $1
            "#,
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch featured posts")?;

        Ok(posts)
    }

    async fn get_by_category(
        &self,
        category: &str,
        limit: Option<u32>,
    ) -> Result<Vec<Post>, AppError> {
        let limit = limit.unwrap_or(10).min(50);

        let posts = sqlx::query_as::<_, Post>(
            r#"
            SELECT id, title, slug, content, excerpt, category, tags, featured_image, featured, 
                   published, seo_title, seo_description, seo_keywords, view_count, 
                   published_at, created_at, updated_at
            FROM posts 
            WHERE category = $1 AND published = true
            ORDER BY published_at DESC 
            LIMIT $2
            "#,
        )
        .bind(category)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch posts by category")?;

        Ok(posts)
    }

    async fn get_by_tags(
        &self,
        tags: Vec<String>,
        limit: Option<u32>,
    ) -> Result<Vec<Post>, AppError> {
        let limit = limit.unwrap_or(10).min(50);

        let posts = sqlx::query_as::<_, Post>(
            r#"
            SELECT id, title, slug, content, excerpt, category, tags, featured_image, featured, 
                   published, seo_title, seo_description, seo_keywords, view_count, 
                   published_at, created_at, updated_at
            FROM posts 
            WHERE tags && $1 AND published = true
            ORDER BY published_at DESC 
            LIMIT $2
            "#,
        )
        .bind(&tags)
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch posts by tags")?;

        Ok(posts)
    }

    async fn get_stats(&self) -> Result<PostStats, AppError> {
        let total_posts: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM posts")
            .fetch_one(&self.pool)
            .await
            .context("Failed to count total posts")?;

        let published_posts: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM posts WHERE published = true")
                .fetch_one(&self.pool)
                .await
                .context("Failed to count published posts")?;

        let draft_posts: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM posts WHERE published = false")
                .fetch_one(&self.pool)
                .await
                .context("Failed to count draft posts")?;

        let featured_posts: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM posts WHERE featured = true AND published = true",
        )
        .fetch_one(&self.pool)
        .await
        .context("Failed to count featured posts")?;

        let posts_this_month: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM posts WHERE EXTRACT(MONTH FROM created_at) = EXTRACT(MONTH FROM CURRENT_DATE) AND EXTRACT(YEAR FROM created_at) = EXTRACT(YEAR FROM CURRENT_DATE)"
        )
        .fetch_one(&self.pool)
        .await
        .context("Failed to count posts this month")?;

        let total_views: i64 = sqlx::query_scalar("SELECT COALESCE(SUM(view_count), 0) FROM posts")
            .fetch_one(&self.pool)
            .await
            .context("Failed to sum total views")?;

        Ok(PostStats {
            total_posts,
            published_posts,
            draft_posts,
            featured_posts,
            posts_this_month,
            total_views,
        })
    }

    async fn update_published_status(&self, id: Uuid, published: bool) -> Result<(), AppError> {
        let result = sqlx::query(
            r#"
            UPDATE posts 
            SET published = $1, 
                published_at = CASE 
                    WHEN $1 = true AND published = false THEN NOW()
                    WHEN $1 = false THEN NULL
                    ELSE published_at
                END,
                updated_at = NOW() 
            WHERE id = $2
            "#,
        )
        .bind(published)
        .bind(id)
        .execute(&self.pool)
        .await
        .context("Failed to update published status")?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Post not found".to_string()));
        }

        Ok(())
    }

    async fn increment_view_count(&self, id: Uuid) -> Result<(), AppError> {
        sqlx::query("UPDATE posts SET view_count = view_count + 1 WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .context("Failed to increment view count")?;

        Ok(())
    }

    async fn check_slug_exists(
        &self,
        slug: &str,
        exclude_id: Option<Uuid>,
    ) -> Result<bool, AppError> {
        let query = match exclude_id {
            Some(id) => sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM posts WHERE slug = $1 AND id != $2",
            )
            .bind(slug)
            .bind(id),
            None => sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM posts WHERE slug = $1")
                .bind(slug),
        };

        let count = query
            .fetch_one(&self.pool)
            .await
            .context("Failed to check slug existence")?;

        Ok(count > 0)
    }
}
