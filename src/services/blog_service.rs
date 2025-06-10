use crate::utils::errors::AppError;
use chrono::Utc;
use std::sync::Arc;
use uuid::Uuid;
type Result<T> = std::result::Result<T, AppError>;

use crate::{
    models::post::{
        CreatePostRequest, Post, PostQuery, PostStats, PostsResponse, UpdatePostRequest,
    },
    repositories::post_repository::PostRepositoryTrait,
};

#[async_trait::async_trait]
pub trait BlogServiceTrait: Send + Sync {
    async fn get_all_posts(&self, query: PostQuery) -> Result<PostsResponse>;
    async fn get_post_by_id(&self, id: Uuid) -> Result<Option<Post>>;
    async fn get_post_by_slug(&self, slug: &str) -> Result<Option<Post>>;
    async fn create_post(&self, request: CreatePostRequest) -> Result<Post>;
    async fn update_post(&self, id: Uuid, request: UpdatePostRequest) -> Result<Post>;
    async fn delete_post(&self, id: Uuid) -> Result<()>;
    async fn get_published_posts(&self, limit: Option<u32>) -> Result<Vec<Post>>;
    async fn get_featured_posts(&self, limit: Option<u32>) -> Result<Vec<Post>>;
    async fn get_posts_by_category(&self, category: &str, limit: Option<u32>) -> Result<Vec<Post>>;
    async fn get_posts_by_tags(&self, tags: Vec<String>, limit: Option<u32>) -> Result<Vec<Post>>;
    async fn get_blog_statistics(&self) -> Result<PostStats>;
    async fn publish_post(&self, id: Uuid) -> Result<()>;
    async fn unpublish_post(&self, id: Uuid) -> Result<()>;
    async fn increment_view_count(&self, id: Uuid) -> Result<()>;
}

#[derive(Clone)]
pub struct BlogService {
    repository: Arc<dyn PostRepositoryTrait>,
}

impl BlogService {
    pub fn new(repository: Arc<dyn PostRepositoryTrait>) -> Self {
        Self { repository }
    }
}

#[async_trait::async_trait]
impl BlogServiceTrait for BlogService {
    async fn get_all_posts(&self, query: PostQuery) -> Result<PostsResponse> {
        // Business logic: Apply default pagination
        let query = PostQuery {
            page: query.page.or(Some(1)),
            limit: query.limit.or(Some(10)),
            ..query
        };

        self.repository.find_all(query).await
    }

    async fn get_post_by_id(&self, id: Uuid) -> Result<Option<Post>> {
        self.repository.find_by_id(id).await
    }

    async fn get_post_by_slug(&self, slug: &str) -> Result<Option<Post>> {
        self.repository.find_by_slug(slug).await
    }

    async fn create_post(&self, request: CreatePostRequest) -> Result<Post> {
        // Business logic: Validate post content
        self.validate_post_content(&request.title, &request.content)?;

        // Business logic: Auto-generate slug if empty
        let mut request = request;
        if request.slug.is_empty() {
            request.slug = self.generate_slug(&request.title);
        }

        // Business logic: Validate slug uniqueness
        if self
            .repository
            .check_slug_exists(&request.slug, None)
            .await?
        {
            // Auto-append timestamp to make it unique
            request.slug = format!("{}-{}", request.slug, Utc::now().timestamp());
        }

        // Business logic: Auto-generate SEO fields if empty
        if request.seo_title.is_none()
            || request
                .seo_title
                .as_ref()
                .unwrap_or(&String::new())
                .is_empty()
        {
            request.seo_title = Some(self.generate_seo_title(&request.title));
        }

        if request.seo_description.is_none()
            || request
                .seo_description
                .as_ref()
                .unwrap_or(&String::new())
                .is_empty()
        {
            request.seo_description = Some(self.generate_seo_description(&request.content));
        }

        // Business logic: Extract and set keywords if not provided
        if request.seo_keywords.is_none()
            || request
                .seo_keywords
                .as_ref()
                .unwrap_or(&String::new())
                .is_empty()
        {
            request.seo_keywords = Some(self.extract_keywords(&request.content, &request.tags));
        }

        self.repository.create(request).await
    }

    async fn update_post(&self, id: Uuid, request: UpdatePostRequest) -> Result<Post> {
        // Business logic: Ensure post exists
        if self.repository.find_by_id(id).await?.is_none() {
            return Err(AppError::NotFound("Post not found".to_string()));
        }

        // Business logic: Validate post content
        self.validate_post_content(&request.title, &request.content)?;

        // Business logic: Validate slug uniqueness (excluding current post)
        if self
            .repository
            .check_slug_exists(&request.slug, Some(id))
            .await?
        {
            return Err(AppError::Validation("Slug already exists".to_string()));
        }

        // Business logic: Update SEO fields if they're empty
        let mut request = request;
        if request
            .seo_title
            .as_ref()
            .unwrap_or(&String::new())
            .is_empty()
        {
            request.seo_title = Some(self.generate_seo_title(&request.title));
        }

        if request
            .seo_description
            .as_ref()
            .unwrap_or(&String::new())
            .is_empty()
        {
            request.seo_description = Some(self.generate_seo_description(&request.content));
        }

        if request
            .seo_keywords
            .as_ref()
            .unwrap_or(&String::new())
            .is_empty()
        {
            request.seo_keywords = Some(self.extract_keywords(&request.content, &request.tags));
        }

        self.repository.update(id, request).await
    }

    async fn delete_post(&self, id: Uuid) -> Result<()> {
        // Business logic: Ensure post exists
        if self.repository.find_by_id(id).await?.is_none() {
            return Err(AppError::NotFound("Post not found".to_string()));
        }

        // Business logic: Check if post can be deleted (no dependencies)
        // In a real application, you might check for comments, etc.

        self.repository.delete(id).await
    }

    async fn get_published_posts(&self, limit: Option<u32>) -> Result<Vec<Post>> {
        // Business logic: Apply reasonable limit
        let limit = limit.unwrap_or(10);
        if limit > 100 {
            return Err(AppError::Validation(
                "Limit cannot exceed 100 posts".to_string(),
            ));
        }

        self.repository.get_published(Some(limit)).await
    }

    async fn get_featured_posts(&self, limit: Option<u32>) -> Result<Vec<Post>> {
        // Business logic: Apply reasonable limit for featured posts
        let limit = limit.unwrap_or(5);
        if limit > 20 {
            return Err(AppError::Validation(
                "Featured posts limit cannot exceed 20".to_string(),
            ));
        }

        self.repository.get_featured(Some(limit)).await
    }

    async fn get_posts_by_category(&self, category: &str, limit: Option<u32>) -> Result<Vec<Post>> {
        // Business logic: Validate category
        if category.trim().is_empty() {
            return Err(AppError::Validation("Category cannot be empty".to_string()));
        }

        let limit = limit.unwrap_or(10);
        if limit > 100 {
            return Err(AppError::Validation(
                "Limit cannot exceed 100 posts".to_string(),
            ));
        }

        self.repository.get_by_category(category, Some(limit)).await
    }

    async fn get_posts_by_tags(&self, tags: Vec<String>, limit: Option<u32>) -> Result<Vec<Post>> {
        // Business logic: Validate tags
        if tags.is_empty() {
            return Err(AppError::Validation(
                "At least one tag is required".to_string(),
            ));
        }

        if tags.len() > 10 {
            return Err(AppError::Validation(
                "Cannot search by more than 10 tags".to_string(),
            ));
        }

        let limit = limit.unwrap_or(10);
        if limit > 100 {
            return Err(AppError::Validation(
                "Limit cannot exceed 100 posts".to_string(),
            ));
        }

        self.repository.get_by_tags(tags, Some(limit)).await
    }

    async fn get_blog_statistics(&self) -> Result<PostStats> {
        self.repository.get_stats().await
    }

    async fn publish_post(&self, id: Uuid) -> Result<()> {
        // Business logic: Ensure post exists and is ready for publishing
        let post = self
            .repository
            .find_by_id(id)
            .await?
            .ok_or_else(|| AppError::NotFound("Post not found".to_string()))?;

        // Business logic: Validate post is ready for publishing
        if post.title.trim().is_empty() {
            return Err(AppError::Validation(
                "Cannot publish post without title".to_string(),
            ));
        }

        if post.content.trim().len() < 100 {
            return Err(AppError::Validation(
                "Post content too short for publishing (minimum 100 characters)".to_string(),
            ));
        }

        if post.category.trim().is_empty() {
            return Err(AppError::Validation(
                "Post must have a category before publishing".to_string(),
            ));
        }

        self.repository.update_published_status(id, true).await
    }

    async fn unpublish_post(&self, id: Uuid) -> Result<()> {
        // Business logic: Ensure post exists
        if self.repository.find_by_id(id).await?.is_none() {
            return Err(AppError::NotFound("Post not found".to_string()));
        }

        self.repository.update_published_status(id, false).await
    }

    async fn increment_view_count(&self, id: Uuid) -> Result<()> {
        // Business logic: Only increment for published posts
        if let Some(post) = self.repository.find_by_id(id).await? {
            if post.published {
                self.repository.increment_view_count(id).await?;
            }
        }

        Ok(())
    }
}

impl BlogService {
    fn validate_post_content(&self, title: &str, content: &str) -> Result<()> {
        if title.trim().is_empty() {
            return Err(AppError::Validation(
                "Post title cannot be empty".to_string(),
            ));
        }

        if title.trim().len() < 5 {
            return Err(AppError::Validation(
                "Post title must be at least 5 characters long".to_string(),
            ));
        }

        if title.len() > 200 {
            return Err(AppError::Validation(
                "Post title cannot exceed 200 characters".to_string(),
            ));
        }

        if content.trim().is_empty() {
            return Err(AppError::Validation(
                "Post content cannot be empty".to_string(),
            ));
        }

        if content.trim().len() < 50 {
            return Err(AppError::Validation(
                "Post content must be at least 50 characters long".to_string(),
            ));
        }

        Ok(())
    }

    fn generate_slug(&self, title: &str) -> String {
        title
            .trim()
            .to_lowercase()
            .chars()
            .map(|c| if c.is_alphanumeric() { c } else { '-' })
            .collect::<String>()
            .split('-')
            .filter(|s| !s.is_empty())
            .collect::<Vec<&str>>()
            .join("-")
    }

    fn generate_seo_title(&self, title: &str) -> String {
        if title.len() <= 60 {
            title.to_string()
        } else {
            format!("{}...", &title[..57])
        }
    }

    fn generate_seo_description(&self, content: &str) -> String {
        let clean_content = content
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace() || ".,!?".contains(*c))
            .collect::<String>();

        if clean_content.len() <= 160 {
            clean_content
        } else {
            format!("{}...", &clean_content[..157])
        }
    }

    fn extract_keywords(&self, content: &str, tags: &[String]) -> String {
        let mut keywords = tags.to_vec();

        // Simple keyword extraction (in a real app, you'd use NLP)
        let words: Vec<&str> = content
            .split_whitespace()
            .filter(|word| word.len() > 4)
            .take(5)
            .collect();

        for word in words {
            keywords.push(word.to_lowercase());
        }

        keywords.join(", ")
    }
}
