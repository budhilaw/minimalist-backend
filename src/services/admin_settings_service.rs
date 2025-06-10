use anyhow::Result;
use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

use crate::{
    models::admin_settings::{
        AdminSettings, AdminSettingsRecord, FeatureSettings, GeneralSettings, NotificationSettings,
        SecuritySettings, UpdateSettingsRequest,
    },
    repositories::AdminSettingsRepository,
};

#[async_trait]
pub trait AdminSettingsServiceTrait: Send + Sync {
    async fn get_all_settings(&self) -> Result<AdminSettings>;
    async fn get_setting(&self, key: &str) -> Result<Option<AdminSettingsRecord>>;
    async fn update_settings(
        &self,
        request: UpdateSettingsRequest,
        updated_by: Option<Uuid>,
    ) -> Result<AdminSettings>;
    async fn update_setting(
        &self,
        key: &str,
        value: serde_json::Value,
        updated_by: Option<Uuid>,
    ) -> Result<AdminSettingsRecord>;
    async fn update_general_settings(
        &self,
        settings: GeneralSettings,
        updated_by: Option<Uuid>,
    ) -> Result<AdminSettings>;
    async fn update_feature_settings(
        &self,
        settings: FeatureSettings,
        updated_by: Option<Uuid>,
    ) -> Result<AdminSettings>;
    async fn update_notification_settings(
        &self,
        settings: NotificationSettings,
        updated_by: Option<Uuid>,
    ) -> Result<AdminSettings>;
    async fn update_security_settings(
        &self,
        settings: SecuritySettings,
        updated_by: Option<Uuid>,
    ) -> Result<AdminSettings>;
    async fn reset_to_defaults(&self, updated_by: Option<Uuid>) -> Result<AdminSettings>;
    async fn is_feature_enabled(&self, feature: &str) -> Result<bool>;
    async fn is_maintenance_mode(&self) -> Result<bool>;
    async fn get_maintenance_message(&self) -> Result<String>;
}

pub struct AdminSettingsService {
    repository: Arc<AdminSettingsRepository>,
}

impl AdminSettingsService {
    pub fn new(repository: Arc<AdminSettingsRepository>) -> Self {
        Self { repository }
    }

    // Helper method to validate feature settings
    fn validate_feature_settings(&self, settings: &FeatureSettings) -> Result<()> {
        // Add any business logic validation here
        // For example, ensure certain features are not disabled together

        if !settings.blog_enabled && !settings.portfolio_enabled && !settings.services_enabled {
            return Err(anyhow::anyhow!(
                "At least one main feature (blog, portfolio, or services) must be enabled"
            ));
        }

        Ok(())
    }

    // Helper method to validate security settings
    fn validate_security_settings(&self, settings: &SecuritySettings) -> Result<()> {
        if settings.session_timeout < 5 || settings.session_timeout > 480 {
            return Err(anyhow::anyhow!(
                "Session timeout must be between 5 and 480 minutes"
            ));
        }

        if settings.max_login_attempts < 3 || settings.max_login_attempts > 10 {
            return Err(anyhow::anyhow!(
                "Max login attempts must be between 3 and 10"
            ));
        }

        // Validate IP whitelist format if provided
        for ip in &settings.ip_whitelist {
            if !self.is_valid_ip_or_cidr(ip) {
                return Err(anyhow::anyhow!(
                    "Invalid IP address or CIDR notation: {}",
                    ip
                ));
            }
        }

        Ok(())
    }

    // Helper method to validate IP address or CIDR notation
    fn is_valid_ip_or_cidr(&self, ip_str: &str) -> bool {
        // Basic validation - in a real implementation, use a proper IP validation library
        if ip_str.contains('/') {
            // CIDR notation
            let parts: Vec<&str> = ip_str.split('/').collect();
            if parts.len() != 2 {
                return false;
            }

            // Validate IP part
            if !self.is_valid_ip(parts[0]) {
                return false;
            }

            // Validate prefix length
            if let Ok(prefix) = parts[1].parse::<u8>() {
                prefix <= 32 // For IPv4
            } else {
                false
            }
        } else {
            self.is_valid_ip(ip_str)
        }
    }

    // Helper method to validate IP address
    fn is_valid_ip(&self, ip_str: &str) -> bool {
        ip_str.parse::<std::net::IpAddr>().is_ok()
    }

    // Helper method to get feature-specific settings
    pub async fn get_feature_config(&self, feature: &str) -> Result<serde_json::Value> {
        let settings = self.get_all_settings().await?;

        match feature {
            "comments" => Ok(serde_json::json!({
                "enabled": settings.features.comments_enabled,
                "moderation_required": true, // Could be configurable
                "max_length": 1000 // Could be configurable
            })),
            "portfolio" => Ok(serde_json::json!({
                "enabled": settings.features.portfolio_enabled,
                "max_projects": 50 // Could be configurable
            })),
            "services" => Ok(serde_json::json!({
                "enabled": settings.features.services_enabled,
                "max_services": 20 // Could be configurable
            })),
            "blog" => Ok(serde_json::json!({
                "enabled": settings.features.blog_enabled,
                "max_posts": 1000 // Could be configurable
            })),
            "contact_form" => Ok(serde_json::json!({
                "enabled": settings.features.contact_form_enabled,
                "rate_limit": "5/hour" // Could be configurable
            })),
            "search" => Ok(serde_json::json!({
                "enabled": settings.features.search_enabled,
                "index_content": true // Could be configurable
            })),
            _ => Err(anyhow::anyhow!("Unknown feature: {}", feature)),
        }
    }
}

#[async_trait]
impl AdminSettingsServiceTrait for AdminSettingsService {
    async fn get_all_settings(&self) -> Result<AdminSettings> {
        self.repository.get_all_settings().await
    }

    async fn get_setting(&self, key: &str) -> Result<Option<AdminSettingsRecord>> {
        self.repository.get_setting(key).await
    }

    async fn update_settings(
        &self,
        request: UpdateSettingsRequest,
        updated_by: Option<Uuid>,
    ) -> Result<AdminSettings> {
        // Validate each section if provided
        if let Some(ref features) = request.features {
            self.validate_feature_settings(features)?;
        }

        if let Some(ref security) = request.security {
            self.validate_security_settings(security)?;
        }

        // Update each section that was provided
        if let Some(general) = request.general {
            self.repository
                .update_general_settings(general, updated_by)
                .await?;
        }

        if let Some(features) = request.features {
            self.repository
                .update_feature_settings(features, updated_by)
                .await?;
        }

        if let Some(notifications) = request.notifications {
            self.repository
                .update_notification_settings(notifications, updated_by)
                .await?;
        }

        if let Some(security) = request.security {
            self.repository
                .update_security_settings(security, updated_by)
                .await?;
        }

        // Return the updated settings
        self.repository.get_all_settings().await
    }

    async fn update_setting(
        &self,
        key: &str,
        value: serde_json::Value,
        updated_by: Option<Uuid>,
    ) -> Result<AdminSettingsRecord> {
        // Validate the setting based on its key
        match key {
            "features" => {
                let features: FeatureSettings = serde_json::from_value(value.clone())?;
                self.validate_feature_settings(&features)?;
            }
            "security" => {
                let security: SecuritySettings = serde_json::from_value(value.clone())?;
                self.validate_security_settings(&security)?;
            }
            "general" | "notifications" => {
                // Basic validation for these settings
                if !value.is_object() {
                    return Err(anyhow::anyhow!("Setting value must be a JSON object"));
                }
            }
            _ => {
                return Err(anyhow::anyhow!("Unknown setting key: {}", key));
            }
        }

        self.repository.update_setting(key, value, updated_by).await
    }

    async fn update_general_settings(
        &self,
        settings: GeneralSettings,
        updated_by: Option<Uuid>,
    ) -> Result<AdminSettings> {
        // Validate general settings
        if settings.site_name.trim().is_empty() {
            return Err(anyhow::anyhow!("Site name cannot be empty"));
        }

        if settings.site_description.len() > 500 {
            return Err(anyhow::anyhow!(
                "Site description cannot exceed 500 characters"
            ));
        }

        if settings.maintenance_message.len() > 1000 {
            return Err(anyhow::anyhow!(
                "Maintenance message cannot exceed 1000 characters"
            ));
        }

        self.repository
            .update_general_settings(settings, updated_by)
            .await
    }

    async fn update_feature_settings(
        &self,
        settings: FeatureSettings,
        updated_by: Option<Uuid>,
    ) -> Result<AdminSettings> {
        self.validate_feature_settings(&settings)?;
        self.repository
            .update_feature_settings(settings, updated_by)
            .await
    }

    async fn update_notification_settings(
        &self,
        settings: NotificationSettings,
        updated_by: Option<Uuid>,
    ) -> Result<AdminSettings> {
        // No specific validation needed for notification settings currently
        self.repository
            .update_notification_settings(settings, updated_by)
            .await
    }

    async fn update_security_settings(
        &self,
        settings: SecuritySettings,
        updated_by: Option<Uuid>,
    ) -> Result<AdminSettings> {
        self.validate_security_settings(&settings)?;
        self.repository
            .update_security_settings(settings, updated_by)
            .await
    }

    async fn reset_to_defaults(&self, updated_by: Option<Uuid>) -> Result<AdminSettings> {
        self.repository.reset_to_defaults(updated_by).await
    }

    async fn is_feature_enabled(&self, feature: &str) -> Result<bool> {
        self.repository.is_feature_enabled(feature).await
    }

    async fn is_maintenance_mode(&self) -> Result<bool> {
        self.repository.is_maintenance_mode().await
    }

    async fn get_maintenance_message(&self) -> Result<String> {
        self.repository.get_maintenance_message().await
    }
}
