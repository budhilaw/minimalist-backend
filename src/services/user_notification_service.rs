use crate::{
    models::user_notification::{
        MarkNotificationReadRequest, MarkNotificationsReadRequest, NotificationStats,
        UpdateNotificationPreferenceRequest,
        UserNotificationPreference, UserNotificationRead, UserNotificationsResponse,
    },
    repositories::UserNotificationRepository,
    utils::errors::AppError,
};
use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

#[async_trait]
pub trait UserNotificationServiceTrait: Send + Sync {
    async fn get_user_notifications(
        &self,
        user_id: Uuid,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<UserNotificationsResponse>;
    async fn mark_notification_read(
        &self,
        user_id: Uuid,
        request: MarkNotificationReadRequest,
    ) -> Result<UserNotificationRead>;
    async fn mark_notifications_read(
        &self,
        user_id: Uuid,
        request: MarkNotificationsReadRequest,
    ) -> Result<i64>;
    async fn mark_all_notifications_read(&self, user_id: Uuid) -> Result<i64>;
    async fn get_notification_stats(&self, user_id: Uuid) -> Result<NotificationStats>;
    async fn get_unread_count(&self, user_id: Uuid) -> Result<i64>;
    async fn get_user_preferences(&self, user_id: Uuid) -> Result<Vec<UserNotificationPreference>>;
    async fn update_notification_preference(
        &self,
        user_id: Uuid,
        request: UpdateNotificationPreferenceRequest,
    ) -> Result<UserNotificationPreference>;
    async fn initialize_user_preferences(&self, user_id: Uuid) -> Result<()>;
}

pub struct UserNotificationService {
    repository: Arc<UserNotificationRepository>,
}

impl UserNotificationService {
    pub fn new(repository: Arc<UserNotificationRepository>) -> Self {
        Self { repository }
    }

    fn validate_notification_type(&self, notification_type: &str) -> Result<(), AppError> {
        let valid_types = vec![
            "login",
            "logout",
            "post_created",
            "post_updated",
            "post_published",
            "portfolio_created",
            "portfolio_updated",
            "service_created",
            "service_updated",
            "comment_approved",
            "comment_rejected",
            "settings_updated",
            "profile_updated",
            "error",
            "warning",
            "system_alert",
        ];

        if !valid_types.contains(&notification_type) {
            return Err(AppError::Validation(format!(
                "Invalid notification type: {}. Valid types are: {}",
                notification_type,
                valid_types.join(", ")
            )));
        }

        Ok(())
    }

    fn validate_delivery_method(&self, delivery_method: &str) -> Result<(), AppError> {
        let valid_methods = ["in_app", "email", "both"];

        if !valid_methods.contains(&delivery_method) {
            return Err(AppError::Validation(format!(
                "Invalid delivery method: {}. Valid methods are: {}",
                delivery_method,
                valid_methods.join(", ")
            )));
        }

        Ok(())
    }
}

#[async_trait]
impl UserNotificationServiceTrait for UserNotificationService {
    async fn get_user_notifications(
        &self,
        user_id: Uuid,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<UserNotificationsResponse> {
        // Validate limits
        let limit = match limit {
            Some(l) if l > 100 => 100, // Cap at 100
            Some(l) if l < 1 => 10,    // Minimum 10
            Some(l) => l,
            None => 20, // Default
        };

        let offset = offset.unwrap_or(0);

        // Get notifications with read status
        let notifications = self
            .repository
            .get_notifications_with_read_status(user_id, Some(limit), Some(offset))
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        // Get statistics
        let stats = self
            .repository
            .get_notification_stats(user_id)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        // Get user preferences
        let preferences = self
            .repository
            .get_user_preferences(user_id)
            .await
            .map_err(|e| AppError::Internal(e.to_string()))?;

        Ok(UserNotificationsResponse {
            notifications,
            stats,
            preferences,
        })
    }

    async fn mark_notification_read(
        &self,
        user_id: Uuid,
        request: MarkNotificationReadRequest,
    ) -> Result<UserNotificationRead> {
        self.repository
            .mark_notification_read(user_id, request.audit_log_id)
            .await
            .map_err(|e| AppError::Internal(e.to_string()).into())
    }

    async fn mark_notifications_read(
        &self,
        user_id: Uuid,
        request: MarkNotificationsReadRequest,
    ) -> Result<i64> {
        // Validate request
        if request.audit_log_ids.is_empty() {
            return Err(AppError::Validation("No audit log IDs provided".to_string()).into());
        }

        if request.audit_log_ids.len() > 100 {
            return Err(AppError::Validation(
                "Cannot mark more than 100 notifications at once".to_string(),
            )
            .into());
        }

        self.repository
            .mark_notifications_read(user_id, request.audit_log_ids)
            .await
            .map_err(|e| AppError::Internal(e.to_string()).into())
    }

    async fn mark_all_notifications_read(&self, user_id: Uuid) -> Result<i64> {
        self.repository
            .mark_all_notifications_read(user_id)
            .await
            .map_err(|e| AppError::Internal(e.to_string()).into())
    }

    async fn get_notification_stats(&self, user_id: Uuid) -> Result<NotificationStats> {
        self.repository
            .get_notification_stats(user_id)
            .await
            .map_err(|e| AppError::Internal(e.to_string()).into())
    }

    async fn get_unread_count(&self, user_id: Uuid) -> Result<i64> {
        self.repository
            .get_unread_count(user_id)
            .await
            .map_err(|e| AppError::Internal(e.to_string()).into())
    }

    async fn get_user_preferences(&self, user_id: Uuid) -> Result<Vec<UserNotificationPreference>> {
        self.repository
            .get_user_preferences(user_id)
            .await
            .map_err(|e| AppError::Internal(e.to_string()).into())
    }

    async fn update_notification_preference(
        &self,
        user_id: Uuid,
        mut request: UpdateNotificationPreferenceRequest,
    ) -> Result<UserNotificationPreference> {
        // Validate notification type
        self.validate_notification_type(&request.notification_type)?;

        // Validate delivery method if provided
        if let Some(ref delivery_method) = request.delivery_method {
            self.validate_delivery_method(delivery_method)?;
        } else {
            request.delivery_method = Some("in_app".to_string());
        }

        self.repository
            .update_notification_preference(user_id, request)
            .await
            .map_err(|e| AppError::Internal(e.to_string()).into())
    }

    async fn initialize_user_preferences(&self, user_id: Uuid) -> Result<()> {
        self.repository
            .initialize_user_preferences(user_id)
            .await
            .map_err(|e| AppError::Internal(e.to_string()).into())
    }
}
 