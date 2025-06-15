use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::{error, info};
use uuid::Uuid;
use validator::Validate;

use crate::{
    models::post::{CreatePostRequest, PostQuery, UpdatePostRequest},
    services::blog_service::BlogServiceTrait,
    utils::errors::AppError,
};

#[derive(Clone)]
pub struct PostState {
    pub blog_service: Arc<dyn BlogServiceTrait>,
}

// GET /api/v1/posts
pub async fn get_all_posts(
    State(state): State<PostState>,
    Query(query): Query<PostQuery>,
) -> Result<Json<Value>, AppError> {
    let response = state.blog_service.get_all_posts(query).await?;
    Ok(Json(json!(response)))
}

// GET /api/v1/posts/:id
pub async fn get_post(
    State(state): State<PostState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, AppError> {
    let post = state
        .blog_service
        .get_post_by_id(id)
        .await?
        .ok_or_else(|| AppError::NotFound("Post not found".to_string()))?;

    // Increment view count
    let _ = state.blog_service.increment_view_count(id).await;

    Ok(Json(json!(post)))
}

// GET /api/v1/posts/slug/:slug
pub async fn get_post_by_slug(
    State(state): State<PostState>,
    Path(slug): Path<String>,
    Query(query): Query<serde_json::Value>,
) -> Result<Json<Value>, AppError> {
    let post = state
        .blog_service
        .get_post_by_slug(&slug)
        .await?
        .ok_or_else(|| AppError::NotFound("Post not found".to_string()))?;

    // Check if this is a preview request
    let is_preview = query
        .get("preview")
        .and_then(|v| v.as_str())
        .map(|s| s == "true")
        .unwrap_or(false);

    // If not in preview mode and post is not published, return 404
    if !is_preview && !post.published {
        return Err(AppError::NotFound("Post not found".to_string()));
    }

    // Only increment view count for published posts (not previews)
    if post.published && !is_preview {
        let _ = state.blog_service.increment_view_count(post.id).await;
    }

    Ok(Json(json!(post)))
}

// POST /api/v1/posts
pub async fn create_post(
    State(state): State<PostState>,
    Json(payload): Json<CreatePostRequest>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    // Validate the request
    payload
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let post = state.blog_service.create_post(payload).await?;

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "message": "Post created successfully",
            "post": post
        })),
    ))
}

// PUT /api/v1/posts/:id
pub async fn update_post(
    State(state): State<PostState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdatePostRequest>,
) -> Result<Json<Value>, AppError> {
    // Validate the request
    payload
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let post = state.blog_service.update_post(id, payload).await?;

    Ok(Json(json!({
        "message": "Post updated successfully",
        "post": post
    })))
}

// DELETE /api/v1/posts/:id
pub async fn delete_post(
    State(state): State<PostState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, AppError> {
    state.blog_service.delete_post(id).await?;

    Ok(Json(json!({
        "message": "Post deleted successfully"
    })))
}

// GET /api/v1/posts/published
pub async fn get_published_posts(
    State(state): State<PostState>,
    Query(query): Query<serde_json::Value>,
) -> Result<Json<Value>, AppError> {
    info!(
        "get_published_posts: Starting request with query: {:?}",
        query
    );

    let limit = query
        .get("limit")
        .and_then(|v| v.as_u64())
        .map(|v| v as u32);

    info!("get_published_posts: Parsed limit: {:?}", limit);

    info!("get_published_posts: Calling blog_service.get_published_posts");
    let posts = match state.blog_service.get_published_posts(limit).await {
        Ok(posts) => {
            info!(
                "get_published_posts: Successfully fetched {} posts",
                posts.len()
            );
            posts
        }
        Err(e) => {
            error!("get_published_posts: Error fetching posts: {:?}", e);
            return Err(e);
        }
    };

    let response = json!({
        "posts": posts,
        "total": posts.len()
    });

    info!(
        "get_published_posts: Returning response with {} posts",
        posts.len()
    );
    Ok(Json(response))
}

// GET /api/v1/posts/featured
pub async fn get_featured_posts(
    State(state): State<PostState>,
    Query(query): Query<serde_json::Value>,
) -> Result<Json<Value>, AppError> {
    let limit = query
        .get("limit")
        .and_then(|v| v.as_u64())
        .map(|v| v as u32);

    let posts = state.blog_service.get_featured_posts(limit).await?;

    Ok(Json(json!({
        "posts": posts,
        "total": posts.len()
    })))
}

// GET /api/v1/posts/category/:category
pub async fn get_posts_by_category(
    State(state): State<PostState>,
    Path(category): Path<String>,
    Query(query): Query<serde_json::Value>,
) -> Result<Json<Value>, AppError> {
    let limit = query
        .get("limit")
        .and_then(|v| v.as_u64())
        .map(|v| v as u32);

    let posts = state
        .blog_service
        .get_posts_by_category(&category, limit)
        .await?;

    Ok(Json(json!({
        "posts": posts,
        "category": category,
        "total": posts.len()
    })))
}

// POST /api/v1/posts/tags
pub async fn get_posts_by_tags(
    State(state): State<PostState>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, AppError> {
    let tags = payload
        .get("tags")
        .and_then(|v| v.as_array())
        .ok_or_else(|| AppError::Validation("Tags array is required".to_string()))?
        .iter()
        .filter_map(|v| v.as_str())
        .map(|s| s.to_string())
        .collect::<Vec<String>>();

    let limit = payload
        .get("limit")
        .and_then(|v| v.as_u64())
        .map(|v| v as u32);

    let posts = state
        .blog_service
        .get_posts_by_tags(tags.clone(), limit)
        .await?;

    Ok(Json(json!({
        "posts": posts,
        "tags": tags,
        "total": posts.len()
    })))
}

// GET /api/v1/posts/stats
pub async fn get_post_stats(State(state): State<PostState>) -> Result<Json<Value>, AppError> {
    let stats = state.blog_service.get_blog_statistics().await?;
    Ok(Json(json!(stats)))
}

// PUT /api/v1/posts/:id/publish
pub async fn update_published_status(
    State(state): State<PostState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, AppError> {
    let published = payload
        .get("published")
        .and_then(|v| v.as_bool())
        .ok_or_else(|| AppError::Validation("Published status is required".to_string()))?;

    if published {
        state.blog_service.publish_post(id).await?;
    } else {
        state.blog_service.unpublish_post(id).await?;
    }

    Ok(Json(json!({
        "message": "Published status updated successfully"
    })))
}
