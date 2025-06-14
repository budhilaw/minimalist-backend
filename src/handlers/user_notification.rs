use axum::{
    extract::{Query, State},
    response::Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;


use crate::{
    models::user_notification::{
        MarkNotificationReadRequest, MarkNotificationsReadRequest,
        UpdateNotificationPreferenceRequest,
    },
    services::{auth_service::Claims, user_notification_service::UserNotificationServiceTrait},
    utils::errors::AppError,
};

#[derive(Clone)]
pub struct UserNotificationState {
    pub user_notification_service: Arc<dyn UserNotificationServiceTrait>,
}

#[derive(Debug, Deserialize)]
pub struct NotificationQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
    pub unread_only: Option<bool>,
}

// GET /api/v1/user/notifications
pub async fn get_user_notifications(
    State(state): State<UserNotificationState>,
    claims: Claims,
    Query(query): Query<NotificationQuery>,
) -> Result<Json<Value>, AppError> {
    info!(
        "get_user_notifications: Fetching notifications for user: {}",
        claims.sub
    );

    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Internal("Invalid user ID".to_string()))?;

    let response = state
        .user_notification_service
        .get_user_notifications(user_id, query.limit, query.offset)
        .await?;

    info!(
        "get_user_notifications: Successfully fetched {} notifications",
        response.notifications.len()
    );

    Ok(Json(json!(response)))
}

// POST /api/v1/user/notifications/mark-read
pub async fn mark_notification_read(
    State(state): State<UserNotificationState>,
    claims: Claims,
    Json(payload): Json<MarkNotificationReadRequest>,
) -> Result<Json<Value>, AppError> {
    info!(
        "mark_notification_read: Marking notification {} as read for user: {}",
        payload.audit_log_id, claims.sub
    );

    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Internal("Invalid user ID".to_string()))?;

    let result = state
        .user_notification_service
        .mark_notification_read(user_id, payload)
        .await?;

    info!("mark_notification_read: Successfully marked notification as read");

    Ok(Json(json!({
        "message": "Notification marked as read",
        "read_record": result
    })))
}

// POST /api/v1/user/notifications/mark-multiple-read
pub async fn mark_notifications_read(
    State(state): State<UserNotificationState>,
    claims: Claims,
    Json(payload): Json<MarkNotificationsReadRequest>,
) -> Result<Json<Value>, AppError> {
    info!(
        "mark_notifications_read: Marking {} notifications as read for user: {}",
        payload.audit_log_ids.len(),
        claims.sub
    );

    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Internal("Invalid user ID".to_string()))?;

    let count = state
        .user_notification_service
        .mark_notifications_read(user_id, payload)
        .await?;

    info!(
        "mark_notifications_read: Successfully marked {} notifications as read",
        count
    );

    Ok(Json(json!({
        "message": format!("Marked {} notifications as read", count),
        "count": count
    })))
}

// POST /api/v1/user/notifications/mark-all-read
pub async fn mark_all_notifications_read(
    State(state): State<UserNotificationState>,
    claims: Claims,
) -> Result<Json<Value>, AppError> {
    info!(
        "mark_all_notifications_read: Marking all notifications as read for user: {}",
        claims.sub
    );

    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Internal("Invalid user ID".to_string()))?;

    let count = state
        .user_notification_service
        .mark_all_notifications_read(user_id)
        .await?;

    info!(
        "mark_all_notifications_read: Successfully marked {} notifications as read",
        count
    );

    Ok(Json(json!({
        "message": format!("Marked {} notifications as read", count),
        "count": count
    })))
}

// GET /api/v1/user/notifications/stats
pub async fn get_notification_stats(
    State(state): State<UserNotificationState>,
    claims: Claims,
) -> Result<Json<Value>, AppError> {
    info!(
        "get_notification_stats: Fetching notification stats for user: {}",
        claims.sub
    );

    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Internal("Invalid user ID".to_string()))?;

    let stats = state
        .user_notification_service
        .get_notification_stats(user_id)
        .await?;

    info!("get_notification_stats: Successfully fetched notification stats");

    Ok(Json(json!(stats)))
}

// GET /api/v1/user/notifications/unread-count
pub async fn get_unread_count(
    State(state): State<UserNotificationState>,
    claims: Claims,
) -> Result<Json<Value>, AppError> {
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Internal("Invalid user ID".to_string()))?;

    let count = state
        .user_notification_service
        .get_unread_count(user_id)
        .await?;

    Ok(Json(json!({
        "unread_count": count
    })))
}

// GET /api/v1/user/notifications/preferences
pub async fn get_notification_preferences(
    State(state): State<UserNotificationState>,
    claims: Claims,
) -> Result<Json<Value>, AppError> {
    info!(
        "get_notification_preferences: Fetching notification preferences for user: {}",
        claims.sub
    );

    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Internal("Invalid user ID".to_string()))?;

    let preferences = state
        .user_notification_service
        .get_user_preferences(user_id)
        .await?;

    info!(
        "get_notification_preferences: Successfully fetched {} preferences",
        preferences.len()
    );

    Ok(Json(json!({
        "preferences": preferences
    })))
}

// PUT /api/v1/user/notifications/preferences
pub async fn update_notification_preference(
    State(state): State<UserNotificationState>,
    claims: Claims,
    Json(payload): Json<UpdateNotificationPreferenceRequest>,
) -> Result<Json<Value>, AppError> {
    info!(
        "update_notification_preference: Updating preference '{}' for user: {}",
        payload.notification_type, claims.sub
    );

    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Internal("Invalid user ID".to_string()))?;

    let preference = state
        .user_notification_service
        .update_notification_preference(user_id, payload)
        .await?;

    info!("update_notification_preference: Successfully updated preference");

    Ok(Json(json!({
        "message": "Notification preference updated successfully",
        "preference": preference
    })))
}
 