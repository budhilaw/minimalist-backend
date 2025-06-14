use crate::models::user_notification::{
    NotificationStats, NotificationWithReadStatus, UpdateNotificationPreferenceRequest,
    UserNotificationPreference, UserNotificationRead,
};
use anyhow::Result;

use sqlx::PgPool;
use uuid::Uuid;

pub struct UserNotificationRepository {
    pool: PgPool,
}

impl UserNotificationRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    // Mark a single notification as read
    pub async fn mark_notification_read(
        &self,
        user_id: Uuid,
        audit_log_id: Uuid,
    ) -> Result<UserNotificationRead> {
        let record = sqlx::query_as!(
            UserNotificationRead,
            r#"
            INSERT INTO user_notification_reads (user_id, audit_log_id)
            VALUES ($1, $2)
            ON CONFLICT (user_id, audit_log_id) DO UPDATE SET
                read_at = NOW()
            RETURNING id, user_id, audit_log_id, read_at, created_at
            "#,
            user_id,
            audit_log_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(record)
    }

    // Mark multiple notifications as read
    pub async fn mark_notifications_read(
        &self,
        user_id: Uuid,
        audit_log_ids: Vec<Uuid>,
    ) -> Result<i64> {
        if audit_log_ids.is_empty() {
            return Ok(0);
        }

        let mut tx = self.pool.begin().await?;

        let mut count = 0i64;
        for audit_log_id in audit_log_ids {
            let result = sqlx::query!(
                r#"
                INSERT INTO user_notification_reads (user_id, audit_log_id)
                VALUES ($1, $2)
                ON CONFLICT (user_id, audit_log_id) DO UPDATE SET
                    read_at = NOW()
                "#,
                user_id,
                audit_log_id
            )
            .execute(&mut *tx)
            .await?;

            count += result.rows_affected() as i64;
        }

        tx.commit().await?;
        Ok(count)
    }

    // Mark all notifications as read for a user
    pub async fn mark_all_notifications_read(&self, user_id: Uuid) -> Result<i64> {
        let result = sqlx::query!(
            r#"
            INSERT INTO user_notification_reads (user_id, audit_log_id)
            SELECT $1, al.id
            FROM audit_logs al
            WHERE NOT EXISTS (
                SELECT 1 FROM user_notification_reads unr 
                WHERE unr.user_id = $1 AND unr.audit_log_id = al.id
            )
            "#,
            user_id
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() as i64)
    }

    // Get notifications with read status for a user
    pub async fn get_notifications_with_read_status(
        &self,
        user_id: Uuid,
        limit: Option<i64>,
        offset: Option<i64>,
    ) -> Result<Vec<NotificationWithReadStatus>> {
        let limit = limit.unwrap_or(50);
        let offset = offset.unwrap_or(0);

        let records = sqlx::query!(
            r#"
            SELECT 
                al.id,
                al.user_id,
                al.user_name,
                al.action,
                al.resource_type,
                al.resource_id,
                al.resource_title,
                al.details,
                al.success,
                al.error_message,
                al.created_at,
                CASE WHEN unr.id IS NOT NULL THEN true ELSE false END as read,
                unr.read_at as "read_at?"
            FROM audit_logs al
            LEFT JOIN user_notification_reads unr ON al.id = unr.audit_log_id AND unr.user_id = $1
            ORDER BY al.created_at DESC
            LIMIT $2 OFFSET $3
            "#,
            user_id,
            limit,
            offset
        )
        .fetch_all(&self.pool)
        .await?;

        let notifications = records
            .into_iter()
            .map(|record| NotificationWithReadStatus {
                id: record.id,
                user_id: record.user_id,
                user_name: record.user_name,
                action: record.action,
                resource_type: record.resource_type,
                resource_id: record.resource_id,
                resource_title: record.resource_title,
                details: record.details,
                success: record.success,
                error_message: record.error_message,
                created_at: record.created_at,
                read: record.read.unwrap_or(false),
                read_at: record.read_at,
            })
            .collect();

        Ok(notifications)
    }

    // Get notification statistics for a user
    pub async fn get_notification_stats(&self, user_id: Uuid) -> Result<NotificationStats> {
        let stats = sqlx::query!(
            r#"
            SELECT 
                COUNT(al.id) as total_notifications,
                COUNT(al.id) - COUNT(unr.id) as unread_notifications,
                COUNT(unr.id) as read_notifications,
                COUNT(CASE WHEN al.created_at >= CURRENT_DATE THEN 1 END) as notifications_today,
                MAX(unr.read_at) as last_read_at
            FROM audit_logs al
            LEFT JOIN user_notification_reads unr ON al.id = unr.audit_log_id AND unr.user_id = $1
            "#,
            user_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(NotificationStats {
            total_notifications: stats.total_notifications.unwrap_or(0),
            unread_notifications: stats.unread_notifications.unwrap_or(0),
            read_notifications: stats.read_notifications.unwrap_or(0),
            notifications_today: stats.notifications_today.unwrap_or(0),
            last_read_at: stats.last_read_at,
        })
    }

    // Get user notification preferences
    pub async fn get_user_preferences(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<UserNotificationPreference>> {
        let preferences = sqlx::query_as!(
            UserNotificationPreference,
            r#"
            SELECT id, user_id, notification_type, enabled, delivery_method, created_at, updated_at
            FROM user_notification_preferences
            WHERE user_id = $1
            ORDER BY notification_type
            "#,
            user_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(preferences)
    }

    // Update user notification preference
    pub async fn update_notification_preference(
        &self,
        user_id: Uuid,
        request: UpdateNotificationPreferenceRequest,
    ) -> Result<UserNotificationPreference> {
        let delivery_method = request
            .delivery_method
            .unwrap_or_else(|| "in_app".to_string());

        let preference = sqlx::query_as!(
            UserNotificationPreference,
            r#"
            INSERT INTO user_notification_preferences (user_id, notification_type, enabled, delivery_method)
            VALUES ($1, $2, $3, $4)
            ON CONFLICT (user_id, notification_type) DO UPDATE SET
                enabled = EXCLUDED.enabled,
                delivery_method = EXCLUDED.delivery_method,
                updated_at = NOW()
            RETURNING id, user_id, notification_type, enabled, delivery_method, created_at, updated_at
            "#,
            user_id,
            request.notification_type,
            request.enabled,
            delivery_method
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(preference)
    }

    // Initialize default preferences for a new user
    pub async fn initialize_user_preferences(&self, user_id: Uuid) -> Result<()> {
        let default_types = vec![
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

        let mut tx = self.pool.begin().await?;

        for notification_type in default_types {
            sqlx::query!(
                r#"
                INSERT INTO user_notification_preferences (user_id, notification_type, enabled, delivery_method)
                VALUES ($1, $2, true, 'in_app')
                ON CONFLICT (user_id, notification_type) DO NOTHING
                "#,
                user_id,
                notification_type
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    // Check if a user has any unread notifications
    pub async fn has_unread_notifications(&self, user_id: Uuid) -> Result<bool> {
        let count = sqlx::query_scalar!(
            r#"
            SELECT COUNT(al.id) as unread_count
            FROM audit_logs al
            LEFT JOIN user_notification_reads unr ON al.id = unr.audit_log_id AND unr.user_id = $1
            WHERE unr.id IS NULL
            "#,
            user_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(count.unwrap_or(0) > 0)
    }

    // Get unread notification count
    pub async fn get_unread_count(&self, user_id: Uuid) -> Result<i64> {
        let count = sqlx::query_scalar!(
            r#"
            SELECT COUNT(al.id) as unread_count
            FROM audit_logs al
            LEFT JOIN user_notification_reads unr ON al.id = unr.audit_log_id AND unr.user_id = $1
            WHERE unr.id IS NULL
            "#,
            user_id
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(count.unwrap_or(0))
    }

    // Clean up old read notifications (older than 30 days)
    pub async fn cleanup_old_read_notifications(&self) -> Result<i64> {
        let result = sqlx::query!(
            r#"
            DELETE FROM user_notification_reads
            WHERE read_at < NOW() - INTERVAL '30 days'
            "#
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected() as i64)
    }
}
