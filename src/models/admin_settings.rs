use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AdminSettingsRecord {
    pub id: Uuid,
    pub setting_key: String,
    pub setting_value: serde_json::Value,
    pub description: Option<String>,
    pub updated_by: Option<Uuid>,
    pub updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdminSettings {
    pub id: String,
    pub general: GeneralSettings,
    pub features: FeatureSettings,
    pub notifications: NotificationSettings,
    pub security: SecuritySettings,
    pub updated_at: DateTime<Utc>,
    pub updated_by: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneralSettings {
    #[serde(rename = "siteName")]
    pub site_name: String,
    #[serde(rename = "siteDescription")]
    pub site_description: String,
    #[serde(rename = "maintenanceMode")]
    pub maintenance_mode: bool,
    #[serde(rename = "maintenanceMessage")]
    pub maintenance_message: String,
    pub social_media_links: SocialMediaLinks,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FeatureSettings {
    #[serde(rename = "commentsEnabled")]
    pub comments_enabled: bool,
    #[serde(rename = "portfolioEnabled")]
    pub portfolio_enabled: bool,
    #[serde(rename = "servicesEnabled")]
    pub services_enabled: bool,
    #[serde(rename = "blogEnabled")]
    pub blog_enabled: bool,
    #[serde(rename = "contactFormEnabled")]
    pub contact_form_enabled: bool,
    #[serde(rename = "searchEnabled")]
    pub search_enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NotificationSettings {
    #[serde(rename = "emailNotifications")]
    pub email_notifications: bool,
    #[serde(rename = "newCommentNotifications")]
    pub new_comment_notifications: bool,
    #[serde(rename = "newContactFormNotifications")]
    pub new_contact_form_notifications: bool,
    #[serde(rename = "systemAlertNotifications")]
    pub system_alert_notifications: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecuritySettings {
    #[serde(rename = "requireStrongPasswords")]
    pub require_strong_passwords: bool,
    #[serde(rename = "sessionTimeout")]
    pub session_timeout: i32,
    #[serde(rename = "maxLoginAttempts")]
    pub max_login_attempts: i32,
    #[serde(rename = "twoFactorEnabled")]
    pub two_factor_enabled: bool,
    #[serde(rename = "ipWhitelist")]
    pub ip_whitelist: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSettingsRequest {
    pub general: Option<GeneralSettings>,
    pub features: Option<FeatureSettings>,
    pub notifications: Option<NotificationSettings>,
    pub security: Option<SecuritySettings>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSettingRequest {
    pub setting_key: String,
    pub setting_value: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SocialMediaLinks {
    pub github: Option<String>,
    pub linkedin: Option<String>,
    pub x: Option<String>,
    pub facebook: Option<String>,
    pub instagram: Option<String>,
    pub email: Option<String>,
}

impl Default for GeneralSettings {
    fn default() -> Self {
        Self {
            site_name: "Ericsson Budhilaw".to_string(),
            site_description: "Senior Software Engineer specializing in consulting and freelancing"
                .to_string(),
            maintenance_mode: false,
            maintenance_message:
                "The site is currently under maintenance. Please check back later.".to_string(),
            social_media_links: SocialMediaLinks::default(),
        }
    }
}

impl Default for FeatureSettings {
    fn default() -> Self {
        Self {
            comments_enabled: true,
            portfolio_enabled: true,
            services_enabled: true,
            blog_enabled: true,
            contact_form_enabled: true,
            search_enabled: true,
        }
    }
}

impl Default for NotificationSettings {
    fn default() -> Self {
        Self {
            email_notifications: true,
            new_comment_notifications: true,
            new_contact_form_notifications: true,
            system_alert_notifications: true,
        }
    }
}

impl Default for SecuritySettings {
    fn default() -> Self {
        Self {
            require_strong_passwords: true,
            session_timeout: 60,
            max_login_attempts: 5,
            two_factor_enabled: false,
            ip_whitelist: vec![],
        }
    }
}

impl Default for SocialMediaLinks {
    fn default() -> Self {
        Self {
            github: Some("https://github.com/budhilaw".to_string()),
            linkedin: Some("https://linkedin.com/in/budhilaw".to_string()),
            x: Some("https://x.com/ceritaeric".to_string()),
            facebook: Some("https://facebook.com/ebudhilaw".to_string()),
            instagram: Some("https://instagram.com/ceritaeric".to_string()),
            email: Some("ericsson@budhilaw.com".to_string()),
        }
    }
}

impl Default for AdminSettings {
    fn default() -> Self {
        Self {
            id: "settings_001".to_string(),
            general: GeneralSettings::default(),
            features: FeatureSettings::default(),
            notifications: NotificationSettings::default(),
            security: SecuritySettings::default(),
            updated_at: Utc::now(),
            updated_by: None,
        }
    }
}
