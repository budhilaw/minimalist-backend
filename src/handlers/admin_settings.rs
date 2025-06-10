use axum::{
    extract::{Path, Query, State},
    response::Json,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;
use validator::Validate;

use crate::{
    middleware::rate_limiter::{BlockedIpInfo, RedisRateLimiter},
    models::admin_settings::{
        FeatureSettings, GeneralSettings, NotificationSettings, SecuritySettings,
        UpdateSettingsRequest,
    },
    services::admin_settings_service::AdminSettingsServiceTrait,
    services::auth_service::Claims,
    utils::errors::AppError,
};

#[derive(Clone)]
pub struct AdminSettingsState {
    pub admin_settings_service: Arc<dyn AdminSettingsServiceTrait>,
    pub rate_limiter: Option<Arc<RedisRateLimiter>>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct BlockIpRequest {
    #[validate(length(min = 7, max = 45, message = "Invalid IP address format"))]
    pub ip: String,
    #[validate(length(min = 1, max = 255, message = "Reason is required"))]
    pub reason: String,
    pub permanent: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct SecurityQuery {
    pub page: Option<u32>,
    pub limit: Option<u32>,
    pub status: Option<String>, // "active", "expired", "all"
}

// GET /api/v1/admin/settings
pub async fn get_settings(
    State(state): State<AdminSettingsState>,
) -> Result<Json<Value>, AppError> {
    info!("get_settings: Fetching all admin settings");

    let settings = state.admin_settings_service.get_all_settings().await?;

    info!("get_settings: Successfully fetched admin settings");
    Ok(Json(json!(settings)))
}

// GET /api/v1/admin/settings/:key
pub async fn get_setting(
    State(state): State<AdminSettingsState>,
    Path(key): Path<String>,
) -> Result<Json<Value>, AppError> {
    info!("get_setting: Fetching setting with key: {}", key);

    let setting = state
        .admin_settings_service
        .get_setting(&key)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Setting '{}' not found", key)))?;

    Ok(Json(json!(setting)))
}

// PUT /api/v1/admin/settings
pub async fn update_settings(
    State(state): State<AdminSettingsState>,
    claims: Claims,
    Json(payload): Json<UpdateSettingsRequest>,
) -> Result<Json<Value>, AppError> {
    info!(
        "update_settings: Updating admin settings for user: {}",
        claims.sub
    );

    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Internal("Invalid user ID".to_string()))?;

    let updated_settings = state
        .admin_settings_service
        .update_settings(payload, Some(user_id))
        .await?;

    info!("update_settings: Successfully updated admin settings");
    Ok(Json(json!({
        "message": "Settings updated successfully",
        "settings": updated_settings
    })))
}

// PUT /api/v1/admin/settings/general
pub async fn update_general_settings(
    State(state): State<AdminSettingsState>,
    claims: Claims,
    Json(payload): Json<GeneralSettings>,
) -> Result<Json<Value>, AppError> {
    info!(
        "update_general_settings: Updating general settings for user: {}",
        claims.sub
    );

    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Internal("Invalid user ID".to_string()))?;

    let updated_settings = state
        .admin_settings_service
        .update_general_settings(payload, Some(user_id))
        .await?;

    info!("update_general_settings: Successfully updated general settings");
    Ok(Json(json!({
        "message": "General settings updated successfully",
        "settings": updated_settings
    })))
}

// PUT /api/v1/admin/settings/features
pub async fn update_feature_settings(
    State(state): State<AdminSettingsState>,
    claims: Claims,
    Json(payload): Json<FeatureSettings>,
) -> Result<Json<Value>, AppError> {
    info!(
        "update_feature_settings: Updating feature settings for user: {}",
        claims.sub
    );

    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Internal("Invalid user ID".to_string()))?;

    let updated_settings = state
        .admin_settings_service
        .update_feature_settings(payload, Some(user_id))
        .await?;

    info!("update_feature_settings: Successfully updated feature settings");
    Ok(Json(json!({
        "message": "Feature settings updated successfully",
        "settings": updated_settings
    })))
}

// PUT /api/v1/admin/settings/notifications
pub async fn update_notification_settings(
    State(state): State<AdminSettingsState>,
    claims: Claims,
    Json(payload): Json<NotificationSettings>,
) -> Result<Json<Value>, AppError> {
    info!(
        "update_notification_settings: Updating notification settings for user: {}",
        claims.sub
    );

    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Internal("Invalid user ID".to_string()))?;

    let updated_settings = state
        .admin_settings_service
        .update_notification_settings(payload, Some(user_id))
        .await?;

    info!("update_notification_settings: Successfully updated notification settings");
    Ok(Json(json!({
        "message": "Notification settings updated successfully",
        "settings": updated_settings
    })))
}

// PUT /api/v1/admin/settings/security
pub async fn update_security_settings(
    State(state): State<AdminSettingsState>,
    claims: Claims,
    Json(payload): Json<SecuritySettings>,
) -> Result<Json<Value>, AppError> {
    info!(
        "update_security_settings: Updating security settings for user: {}",
        claims.sub
    );

    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Internal("Invalid user ID".to_string()))?;

    let updated_settings = state
        .admin_settings_service
        .update_security_settings(payload, Some(user_id))
        .await?;

    info!("update_security_settings: Successfully updated security settings");
    Ok(Json(json!({
        "message": "Security settings updated successfully",
        "settings": updated_settings
    })))
}

// POST /api/v1/admin/settings/reset
pub async fn reset_settings(
    State(state): State<AdminSettingsState>,
    claims: Claims,
) -> Result<Json<Value>, AppError> {
    info!(
        "reset_settings: Resetting all settings to defaults for user: {}",
        claims.sub
    );

    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Internal("Invalid user ID".to_string()))?;

    let default_settings = state
        .admin_settings_service
        .reset_to_defaults(Some(user_id))
        .await?;

    info!("reset_settings: Successfully reset all settings to defaults");
    Ok(Json(json!({
        "message": "All settings have been reset to defaults",
        "settings": default_settings
    })))
}

// GET /api/v1/admin/settings/features/:feature/enabled
pub async fn is_feature_enabled(
    State(state): State<AdminSettingsState>,
    Path(feature): Path<String>,
) -> Result<Json<Value>, AppError> {
    let enabled = state
        .admin_settings_service
        .is_feature_enabled(&feature)
        .await?;

    Ok(Json(json!({
        "feature": feature,
        "enabled": enabled
    })))
}

// GET /api/v1/admin/settings/maintenance-mode
pub async fn get_maintenance_mode(
    State(state): State<AdminSettingsState>,
) -> Result<Json<Value>, AppError> {
    let maintenance_mode = state.admin_settings_service.is_maintenance_mode().await?;

    let maintenance_message = if maintenance_mode {
        Some(
            state
                .admin_settings_service
                .get_maintenance_message()
                .await?,
        )
    } else {
        None
    };

    Ok(Json(json!({
        "maintenance_mode": maintenance_mode,
        "maintenance_message": maintenance_message
    })))
}

// PUT /api/v1/admin/settings/:key
pub async fn update_setting(
    State(state): State<AdminSettingsState>,
    claims: Claims,
    Path(key): Path<String>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, AppError> {
    info!(
        "update_setting: Updating setting '{}' for user: {}",
        key, claims.sub
    );

    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Internal("Invalid user ID".to_string()))?;

    let updated_setting = state
        .admin_settings_service
        .update_setting(&key, payload, Some(user_id))
        .await?;

    info!("update_setting: Successfully updated setting '{}'", key);
    Ok(Json(json!({
        "message": format!("Setting '{}' updated successfully", key),
        "setting": updated_setting
    })))
}

// GET /api/v1/admin/settings/security/blocked-ips
pub async fn get_blocked_ips(
    State(state): State<AdminSettingsState>,
    Query(query): Query<SecurityQuery>,
    _claims: Claims,
) -> Result<Json<Value>, AppError> {
    if let Some(ref rate_limiter) = state.rate_limiter {
        let blocked_ips = rate_limiter
            .get_blocked_ips()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to fetch blocked IPs: {}", e)))?;

        // Apply filtering
        let filtered_ips: Vec<&BlockedIpInfo> = match query.status.as_deref() {
            Some("active") => blocked_ips
                .iter()
                .filter(|ip| ip.expires_at.is_none_or(|exp| chrono::Utc::now() < exp))
                .collect(),
            Some("expired") => blocked_ips
                .iter()
                .filter(|ip| ip.expires_at.is_some_and(|exp| chrono::Utc::now() >= exp))
                .collect(),
            _ => blocked_ips.iter().collect(),
        };

        // Apply pagination
        let limit = query.limit.unwrap_or(20).min(100) as usize;
        let page = query.page.unwrap_or(1).max(1) as usize;
        let offset = (page - 1) * limit;

        let total = filtered_ips.len();
        let paginated_ips: Vec<&BlockedIpInfo> =
            filtered_ips.into_iter().skip(offset).take(limit).collect();

        Ok(Json(json!({
            "success": true,
            "data": {
                "blocked_ips": paginated_ips,
                "pagination": {
                    "current_page": page,
                    "total_pages": total.div_ceil(limit),
                    "total_items": total,
                    "items_per_page": limit
                }
            }
        })))
    } else {
        Err(AppError::Internal("Rate limiter not available".to_string()))
    }
}

// POST /api/v1/admin/settings/security/block-ip
pub async fn block_ip(
    State(state): State<AdminSettingsState>,
    _claims: Claims,
    Json(request): Json<BlockIpRequest>,
) -> Result<Json<Value>, AppError> {
    // Validate request
    request
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    if let Some(ref rate_limiter) = state.rate_limiter {
        let permanent = request.permanent.unwrap_or(false);

        rate_limiter
            .block_ip(&request.ip, &request.reason, permanent)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to block IP: {}", e)))?;

        Ok(Json(json!({
            "success": true,
            "message": format!("IP {} has been {}blocked",
                request.ip,
                if permanent { "permanently " } else { "" }
            )
        })))
    } else {
        Err(AppError::Internal("Rate limiter not available".to_string()))
    }
}

// DELETE /api/v1/admin/settings/security/blocked-ips/:ip
pub async fn unblock_ip(
    State(state): State<AdminSettingsState>,
    Path(ip): Path<String>,
    _claims: Claims,
) -> Result<Json<Value>, AppError> {
    if let Some(ref rate_limiter) = state.rate_limiter {
        rate_limiter
            .unblock_ip(&ip)
            .await
            .map_err(|e| AppError::Internal(format!("Failed to unblock IP: {}", e)))?;

        Ok(Json(json!({
            "success": true,
            "message": format!("IP {} has been unblocked", ip)
        })))
    } else {
        Err(AppError::Internal("Rate limiter not available".to_string()))
    }
}

// GET /api/v1/admin/settings/security/stats
pub async fn get_security_stats(
    State(state): State<AdminSettingsState>,
    _claims: Claims,
) -> Result<Json<Value>, AppError> {
    if let Some(ref rate_limiter) = state.rate_limiter {
        let blocked_ips = rate_limiter
            .get_blocked_ips()
            .await
            .map_err(|e| AppError::Internal(format!("Failed to fetch security stats: {}", e)))?;

        let now = chrono::Utc::now();
        let active_blocks = blocked_ips
            .iter()
            .filter(|ip| ip.expires_at.is_none_or(|exp| now < exp))
            .count();

        let permanent_blocks = blocked_ips
            .iter()
            .filter(|ip| ip.expires_at.is_none())
            .count();

        let temporary_blocks = blocked_ips
            .iter()
            .filter(|ip| ip.expires_at.is_some())
            .count();

        // Recent activity (last 24 hours)
        let recent_blocks = blocked_ips
            .iter()
            .filter(|ip| {
                let hours_ago_24 = now - chrono::Duration::hours(24);
                ip.blocked_at > hours_ago_24
            })
            .count();

        Ok(Json(json!({
            "success": true,
            "data": {
                "total_blocked_ips": blocked_ips.len(),
                "active_blocks": active_blocks,
                "permanent_blocks": permanent_blocks,
                "temporary_blocks": temporary_blocks,
                "recent_blocks_24h": recent_blocks,
                "last_updated": now
            }
        })))
    } else {
        Err(AppError::Internal("Rate limiter not available".to_string()))
    }
}
