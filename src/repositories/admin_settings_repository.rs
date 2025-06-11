use crate::models::admin_settings::{
    AdminSettings, AdminSettingsRecord, FeatureSettings, GeneralSettings, NotificationSettings,
    SecuritySettings,
};
use anyhow::{anyhow, Result};
use chrono::Utc;
use sqlx::PgPool;
use uuid::Uuid;

pub struct AdminSettingsRepository {
    pool: PgPool,
}

impl AdminSettingsRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn get_all_settings(&self) -> Result<AdminSettings> {
        let records = sqlx::query_as!(
            AdminSettingsRecord,
            r#"
            SELECT id, setting_key, setting_value, description, updated_by, updated_at, created_at
            FROM admin_settings 
            ORDER BY setting_key
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        self.build_admin_settings(records).await
    }

    /// Initialize default admin settings if they don't exist
    /// This is safe to call multiple times - it won't overwrite existing settings
    pub async fn ensure_settings_exist(&self) -> Result<()> {
        // Check if any settings exist
        let count = sqlx::query_scalar!("SELECT COUNT(*) FROM admin_settings")
            .fetch_one(&self.pool)
            .await?;

        if count.unwrap_or(0) == 0 {
            // No settings exist, create defaults
            let default_settings = AdminSettings::default();

            let general_value = serde_json::to_value(default_settings.general)?;
            let features_value = serde_json::to_value(default_settings.features)?;
            let notifications_value = serde_json::to_value(default_settings.notifications)?;
            let security_value = serde_json::to_value(default_settings.security)?;

            let mut tx = self.pool.begin().await?;

            sqlx::query!(
                "INSERT INTO admin_settings (id, setting_key, setting_value, description, created_at, updated_at) VALUES ($1, $2, $3, $4, NOW(), NOW())",
                uuid::Uuid::new_v4(),
                "general",
                general_value,
                Some("General site settings and configuration")
            )
            .execute(&mut *tx)
            .await?;

            sqlx::query!(
                "INSERT INTO admin_settings (id, setting_key, setting_value, description, created_at, updated_at) VALUES ($1, $2, $3, $4, NOW(), NOW())",
                uuid::Uuid::new_v4(),
                "features",
                features_value,
                Some("Feature toggles and availability")
            )
            .execute(&mut *tx)
            .await?;

            sqlx::query!(
                "INSERT INTO admin_settings (id, setting_key, setting_value, description, created_at, updated_at) VALUES ($1, $2, $3, $4, NOW(), NOW())",
                uuid::Uuid::new_v4(),
                "notifications",
                notifications_value,
                Some("Notification preferences and settings")
            )
            .execute(&mut *tx)
            .await?;

            sqlx::query!(
                "INSERT INTO admin_settings (id, setting_key, setting_value, description, created_at, updated_at) VALUES ($1, $2, $3, $4, NOW(), NOW())",
                uuid::Uuid::new_v4(),
                "security",
                security_value,
                Some("Security and access control settings")
            )
            .execute(&mut *tx)
            .await?;

            tx.commit().await?;

            tracing::info!("✅ Default admin settings initialized");
        } else {
            tracing::info!("📊 Admin settings already exist, skipping initialization");
        }

        Ok(())
    }

    pub async fn get_setting(&self, key: &str) -> Result<Option<AdminSettingsRecord>> {
        let record = sqlx::query_as!(
            AdminSettingsRecord,
            r#"
            SELECT id, setting_key, setting_value, description, updated_by, updated_at, created_at
            FROM admin_settings 
            WHERE setting_key = $1
            "#,
            key
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(record)
    }

    pub async fn update_setting(
        &self,
        key: &str,
        value: serde_json::Value,
        updated_by: Option<Uuid>,
    ) -> Result<AdminSettingsRecord> {
        let record = sqlx::query_as!(
            AdminSettingsRecord,
            r#"
            UPDATE admin_settings 
            SET setting_value = $1, updated_by = $2, updated_at = NOW()
            WHERE setting_key = $3
            RETURNING id, setting_key, setting_value, description, updated_by, updated_at, created_at
            "#,
            value,
            updated_by,
            key
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(record)
    }

    pub async fn update_general_settings(
        &self,
        settings: GeneralSettings,
        updated_by: Option<Uuid>,
    ) -> Result<AdminSettings> {
        let value = serde_json::to_value(settings)?;
        self.update_setting("general", value, updated_by).await?;
        self.get_all_settings().await
    }

    pub async fn update_feature_settings(
        &self,
        settings: FeatureSettings,
        updated_by: Option<Uuid>,
    ) -> Result<AdminSettings> {
        let value = serde_json::to_value(settings)?;
        self.update_setting("features", value, updated_by).await?;
        self.get_all_settings().await
    }

    pub async fn update_notification_settings(
        &self,
        settings: NotificationSettings,
        updated_by: Option<Uuid>,
    ) -> Result<AdminSettings> {
        let value = serde_json::to_value(settings)?;
        self.update_setting("notifications", value, updated_by)
            .await?;
        self.get_all_settings().await
    }

    pub async fn update_security_settings(
        &self,
        settings: SecuritySettings,
        updated_by: Option<Uuid>,
    ) -> Result<AdminSettings> {
        let value = serde_json::to_value(settings)?;
        self.update_setting("security", value, updated_by).await?;
        self.get_all_settings().await
    }

    pub async fn reset_to_defaults(&self, updated_by: Option<Uuid>) -> Result<AdminSettings> {
        let default_settings = AdminSettings::default();

        // Update each setting with default values
        let general_value = serde_json::to_value(default_settings.general)?;
        let features_value = serde_json::to_value(default_settings.features)?;
        let notifications_value = serde_json::to_value(default_settings.notifications)?;
        let security_value = serde_json::to_value(default_settings.security)?;

        // Execute all updates in a transaction
        let mut tx = self.pool.begin().await?;

        sqlx::query!(
            "UPDATE admin_settings SET setting_value = $1, updated_by = $2, updated_at = NOW() WHERE setting_key = 'general'",
            general_value,
            updated_by
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            "UPDATE admin_settings SET setting_value = $1, updated_by = $2, updated_at = NOW() WHERE setting_key = 'features'",
            features_value,
            updated_by
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            "UPDATE admin_settings SET setting_value = $1, updated_by = $2, updated_at = NOW() WHERE setting_key = 'notifications'",
            notifications_value,
            updated_by
        )
        .execute(&mut *tx)
        .await?;

        sqlx::query!(
            "UPDATE admin_settings SET setting_value = $1, updated_by = $2, updated_at = NOW() WHERE setting_key = 'security'",
            security_value,
            updated_by
        )
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;

        self.get_all_settings().await
    }

    pub async fn is_feature_enabled(&self, feature: &str) -> Result<bool> {
        let record = self.get_setting("features").await?;

        if let Some(record) = record {
            let features: FeatureSettings = serde_json::from_value(record.setting_value)?;
            match feature {
                "comments" => Ok(features.comments_enabled),
                "portfolio" => Ok(features.portfolio_enabled),
                "services" => Ok(features.services_enabled),
                "blog" => Ok(features.blog_enabled),
                "contactForm" => Ok(features.contact_form_enabled),
                "search" => Ok(features.search_enabled),
                _ => Err(anyhow!("Unknown feature: {}", feature)),
            }
        } else {
            // Default to true if setting doesn't exist
            Ok(true)
        }
    }

    pub async fn is_maintenance_mode(&self) -> Result<bool> {
        let record = self.get_setting("general").await?;

        if let Some(record) = record {
            let general: GeneralSettings = serde_json::from_value(record.setting_value)?;
            Ok(general.maintenance_mode)
        } else {
            Ok(false)
        }
    }

    pub async fn get_maintenance_message(&self) -> Result<String> {
        let record = self.get_setting("general").await?;

        if let Some(record) = record {
            let general: GeneralSettings = serde_json::from_value(record.setting_value)?;
            Ok(general.maintenance_message)
        } else {
            Ok("The site is currently under maintenance. Please check back later.".to_string())
        }
    }

    async fn build_admin_settings(
        &self,
        records: Vec<AdminSettingsRecord>,
    ) -> Result<AdminSettings> {
        let mut general = GeneralSettings::default();
        let mut features = FeatureSettings::default();
        let mut notifications = NotificationSettings::default();
        let mut security = SecuritySettings::default();
        let mut latest_update = Utc::now();
        let mut updated_by = None;

        for record in records {
            if record.updated_at > latest_update {
                latest_update = record.updated_at;
                updated_by = record.updated_by;
            }

            match record.setting_key.as_str() {
                "general" => {
                    general = serde_json::from_value(record.setting_value)?;
                }
                "features" => {
                    features = serde_json::from_value(record.setting_value)?;
                }
                "notifications" => {
                    notifications = serde_json::from_value(record.setting_value)?;
                }
                "security" => {
                    security = serde_json::from_value(record.setting_value)?;
                }
                _ => {} // Ignore unknown settings
            }
        }

        // Get user name if updated_by is set
        let updated_by_name = if let Some(user_id) = updated_by {
            let user = sqlx::query!("SELECT username FROM users WHERE id = $1", user_id)
                .fetch_optional(&self.pool)
                .await?;

            user.map(|u| u.username)
        } else {
            None
        };

        Ok(AdminSettings {
            id: "settings_001".to_string(),
            general,
            features,
            notifications,
            security,
            updated_at: latest_update,
            updated_by: updated_by_name,
        })
    }
}
