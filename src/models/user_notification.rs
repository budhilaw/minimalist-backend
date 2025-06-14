use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserNotificationRead {
    pub id: Uuid,
    pub user_id: Uuid,
    pub audit_log_id: Uuid,
    pub read_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct UserNotificationPreference {
    pub id: Uuid,
    pub user_id: Uuid,
    pub notification_type: String,
    pub enabled: bool,
    pub delivery_method: String, // 'in_app', 'email', 'both'
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct MarkNotificationReadRequest {
    pub audit_log_id: Uuid,
}

#[derive(Debug, Deserialize, Validate)]
pub struct MarkNotificationsReadRequest {
    pub audit_log_ids: Vec<Uuid>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateNotificationPreferenceRequest {
    pub notification_type: String,
    pub enabled: bool,
    pub delivery_method: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct NotificationWithReadStatus {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub user_name: Option<String>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<Uuid>,
    pub resource_title: Option<String>,
    pub details: Option<String>,
    pub success: bool,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
    pub read: bool,
    pub read_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct NotificationStats {
    pub total_notifications: i64,
    pub unread_notifications: i64,
    pub read_notifications: i64,
    pub notifications_today: i64,
    pub last_read_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct UserNotificationsResponse {
    pub notifications: Vec<NotificationWithReadStatus>,
    pub stats: NotificationStats,
    pub preferences: Vec<UserNotificationPreference>,
}
