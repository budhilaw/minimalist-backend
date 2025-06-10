use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Post {
    pub id: Uuid,
    pub title: String,
    pub slug: String,
    pub content: String,
    pub excerpt: Option<String>,
    pub category: String,
    pub tags: Vec<String>,
    pub featured_image: Option<String>,
    pub featured: bool,
    pub published: bool,
    pub seo_title: Option<String>,
    pub seo_description: Option<String>,
    pub seo_keywords: Option<String>,
    pub view_count: i32,
    pub published_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct PostResponse {
    pub id: Uuid,
    pub title: String,
    pub slug: String,
    pub content: String,
    pub excerpt: Option<String>,
    pub category: String,
    pub tags: Vec<String>,
    pub featured_image: Option<String>,
    pub featured: bool,
    pub published: bool,
    pub seo_title: Option<String>,
    pub seo_description: Option<String>,
    pub seo_keywords: Option<String>,
    pub view_count: i32,
    pub published_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Post> for PostResponse {
    fn from(post: Post) -> Self {
        Self {
            id: post.id,
            title: post.title,
            slug: post.slug,
            content: post.content,
            excerpt: post.excerpt,
            category: post.category,
            tags: post.tags,
            featured_image: post.featured_image,
            featured: post.featured,
            published: post.published,
            seo_title: post.seo_title,
            seo_description: post.seo_description,
            seo_keywords: post.seo_keywords,
            view_count: post.view_count,
            published_at: post.published_at,
            created_at: post.created_at,
            updated_at: post.updated_at,
        }
    }
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreatePostRequest {
    #[validate(length(
        min = 1,
        max = 255,
        message = "Title is required and must be less than 255 characters"
    ))]
    pub title: String,
    #[validate(length(max = 255, message = "Slug must be less than 255 characters"))]
    pub slug: String,
    #[validate(length(min = 1, message = "Content is required"))]
    pub content: String,
    #[validate(length(max = 500, message = "Excerpt must be less than 500 characters"))]
    pub excerpt: Option<String>,
    #[validate(length(
        min = 1,
        max = 100,
        message = "Category is required and must be less than 100 characters"
    ))]
    pub category: String,
    pub tags: Vec<String>,
    #[validate(url(message = "Featured image must be a valid URL"))]
    pub featured_image: Option<String>,
    pub featured: Option<bool>,
    pub published: Option<bool>,
    #[validate(length(max = 255, message = "SEO title must be less than 255 characters"))]
    pub seo_title: Option<String>,
    #[validate(length(
        max = 500,
        message = "SEO description must be less than 500 characters"
    ))]
    pub seo_description: Option<String>,
    pub seo_keywords: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdatePostRequest {
    #[validate(length(
        min = 1,
        max = 255,
        message = "Title is required and must be less than 255 characters"
    ))]
    pub title: String,
    #[validate(length(max = 255, message = "Slug must be less than 255 characters"))]
    pub slug: String,
    #[validate(length(min = 1, message = "Content is required"))]
    pub content: String,
    #[validate(length(max = 500, message = "Excerpt must be less than 500 characters"))]
    pub excerpt: Option<String>,
    #[validate(length(
        min = 1,
        max = 100,
        message = "Category is required and must be less than 100 characters"
    ))]
    pub category: String,
    pub tags: Vec<String>,
    #[validate(url(message = "Featured image must be a valid URL"))]
    pub featured_image: Option<String>,
    pub featured: Option<bool>,
    pub published: Option<bool>,
    #[validate(length(max = 255, message = "SEO title must be less than 255 characters"))]
    pub seo_title: Option<String>,
    #[validate(length(
        max = 500,
        message = "SEO description must be less than 500 characters"
    ))]
    pub seo_description: Option<String>,
    pub seo_keywords: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PostQuery {
    pub page: Option<u32>,
    pub limit: Option<u32>,
    pub category: Option<String>,
    pub search: Option<String>,
    pub published: Option<bool>,
    pub featured: Option<bool>,
    pub author_id: Option<Uuid>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct PostsResponse {
    pub posts: Vec<PostResponse>,
    pub total: i64,
    pub page: u32,
    pub limit: u32,
    pub total_pages: u32,
}

#[derive(Debug, Serialize)]
pub struct PostStats {
    pub total_posts: i64,
    pub published_posts: i64,
    pub draft_posts: i64,
    pub featured_posts: i64,
    pub posts_this_month: i64,
    pub total_views: i64,
}
