use axum::{
    extract::{ConnectInfo, Path, Query, State},
    http::{HeaderMap, StatusCode},
    response::Json,
};
use serde_json::{json, Value};
use std::{net::SocketAddr, sync::Arc};
use uuid::Uuid;
use validator::Validate;

use crate::{
    models::comment::{CommentQuery, CreateCommentRequest, UpdateCommentStatusRequest},
    services::comment_service::CommentServiceTrait,
    utils::errors::AppError,
};

#[derive(Clone)]
pub struct CommentState {
    pub comment_service: Arc<dyn CommentServiceTrait>,
}

// GET /api/v1/comments
pub async fn get_all_comments(
    State(state): State<CommentState>,
    Query(query): Query<CommentQuery>,
) -> Result<Json<Value>, AppError> {
    let response = state.comment_service.get_all_comments(query).await?;
    Ok(Json(json!(response)))
}

// GET /api/v1/comments/:id
pub async fn get_comment(
    State(state): State<CommentState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, AppError> {
    let comment = state
        .comment_service
        .get_comment_by_id(id)
        .await?
        .ok_or_else(|| AppError::NotFound("Comment not found".to_string()))?;

    Ok(Json(json!(comment)))
}

// POST /api/v1/comments
pub async fn create_comment(
    State(state): State<CommentState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(payload): Json<CreateCommentRequest>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    // Validate the request
    payload
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    // Extract IP address and User-Agent
    let ip_address = Some(addr.ip().to_string());
    let user_agent = headers
        .get("user-agent")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string());

    let comment = state
        .comment_service
        .create_comment(payload, ip_address, user_agent)
        .await?;

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "message": "Comment submitted successfully and is pending moderation",
            "comment": comment
        })),
    ))
}

// PUT /api/v1/comments/:id/status
pub async fn update_comment_status(
    State(state): State<CommentState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateCommentStatusRequest>,
) -> Result<Json<Value>, AppError> {
    // Validate the request
    payload
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let comment = state
        .comment_service
        .update_comment_status(id, payload)
        .await?;

    Ok(Json(json!({
        "message": "Comment status updated successfully",
        "comment": comment
    })))
}

// DELETE /api/v1/comments/:id
pub async fn delete_comment(
    State(state): State<CommentState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, AppError> {
    state.comment_service.delete_comment(id).await?;

    Ok(Json(json!({
        "message": "Comment deleted successfully"
    })))
}

// GET /api/v1/comments/post/:post_id
pub async fn get_comments_by_post(
    State(state): State<CommentState>,
    Path(post_id): Path<Uuid>,
    Query(query): Query<serde_json::Value>,
) -> Result<Json<Value>, AppError> {
    let include_replies = query
        .get("include_replies")
        .and_then(|v| v.as_bool())
        .unwrap_or(true);

    let comments = state
        .comment_service
        .get_comments_by_post(post_id, include_replies)
        .await?;

    Ok(Json(json!({
        "comments": comments,
        "post_id": post_id,
        "total": comments.len(),
        "include_replies": include_replies
    })))
}

// GET /api/v1/comments/:id/replies
pub async fn get_comment_replies(
    State(state): State<CommentState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, AppError> {
    let replies = state.comment_service.get_comment_replies(id).await?;

    Ok(Json(json!({
        "replies": replies,
        "parent_id": id,
        "total": replies.len()
    })))
}

// GET /api/v1/comments/pending
pub async fn get_pending_comments(
    State(state): State<CommentState>,
) -> Result<Json<Value>, AppError> {
    let comments = state.comment_service.get_pending_comments().await?;

    Ok(Json(json!({
        "comments": comments,
        "total": comments.len()
    })))
}

// GET /api/v1/comments/stats
pub async fn get_comment_stats(State(state): State<CommentState>) -> Result<Json<Value>, AppError> {
    let stats = state.comment_service.get_comment_statistics().await?;
    Ok(Json(json!(stats)))
}

// PUT /api/v1/comments/bulk-status
pub async fn bulk_update_comment_status(
    State(state): State<CommentState>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, AppError> {
    let ids = payload
        .get("ids")
        .and_then(|v| v.as_array())
        .ok_or_else(|| AppError::Validation("Comment IDs array is required".to_string()))?
        .iter()
        .filter_map(|v| v.as_str())
        .filter_map(|s| Uuid::parse_str(s).ok())
        .collect::<Vec<Uuid>>();

    let status = payload
        .get("status")
        .and_then(|v| v.as_str())
        .ok_or_else(|| AppError::Validation("Status is required".to_string()))?
        .to_string();

    if ids.is_empty() {
        return Err(AppError::Validation(
            "At least one comment ID is required".to_string(),
        ));
    }

    let affected_rows = state
        .comment_service
        .bulk_moderate_comments(ids.clone(), status.clone())
        .await?;

    Ok(Json(json!({
        "message": "Comments updated successfully",
        "affected_rows": affected_rows,
        "status": status,
        "comment_ids": ids
    })))
}

// PUT /api/v1/comments/:id/approve - Quick approve endpoint
pub async fn approve_comment(
    State(state): State<CommentState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, AppError> {
    state.comment_service.approve_comment(id).await?;

    Ok(Json(json!({
        "message": "Comment approved successfully",
        "comment_id": id
    })))
}

// PUT /api/v1/comments/:id/reject - Quick reject endpoint
pub async fn reject_comment(
    State(state): State<CommentState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, AppError> {
    state.comment_service.reject_comment(id).await?;

    Ok(Json(json!({
        "message": "Comment rejected successfully",
        "comment_id": id
    })))
}
