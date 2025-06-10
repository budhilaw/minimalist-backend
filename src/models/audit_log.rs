use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use sqlx::types::ipnetwork::IpNetwork;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct AuditLog {
    pub id: Uuid,
    pub user_id: Option<Uuid>,
    pub user_name: Option<String>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<Uuid>,
    pub resource_title: Option<String>,
    pub details: Option<String>,
    pub old_values: Option<serde_json::Value>,
    pub new_values: Option<serde_json::Value>,
    pub ip_address: Option<IpNetwork>,
    pub user_agent: Option<String>,
    pub success: bool,
    pub error_message: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Deserialize)]
pub struct CreateAuditLogRequest {
    pub user_id: Option<Uuid>,
    pub user_name: Option<String>,
    pub action: String,
    pub resource_type: String,
    pub resource_id: Option<Uuid>,
    pub resource_title: Option<String>,
    pub details: Option<String>,
    pub old_values: Option<serde_json::Value>,
    pub new_values: Option<serde_json::Value>,
    pub ip_address: Option<IpNetwork>,
    pub user_agent: Option<String>,
    pub success: bool,
    pub error_message: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct AuditLogFilters {
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub action: Option<String>,
    pub resource_type: Option<String>,
    pub user_id: Option<Uuid>,
    pub success: Option<bool>,
    pub search: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize)]
pub struct AuditLogResponse {
    pub logs: Vec<AuditLog>,
    pub total_count: i64,
    pub page: i64,
    pub per_page: i64,
    pub total_pages: i64,
}

// Audit action types for type safety
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditAction {
    // Authentication
    Login,
    Logout,
    LoginFailed,
    
    // Posts
    PostCreated,
    PostUpdated,
    PostDeleted,
    PostPublished,
    PostUnpublished,
    
    // Portfolio
    PortfolioCreated,
    PortfolioUpdated,
    PortfolioDeleted,
    PortfolioFeatured,
    PortfolioUnfeatured,
    
    // Services
    ServiceCreated,
    ServiceUpdated,
    ServiceDeleted,
    ServiceActivated,
    ServiceDeactivated,
    
    // Comments
    CommentApproved,
    CommentRejected,
    CommentDeleted,
    
    // Settings
    SettingsUpdated,
    
    // Profile
    ProfileUpdated,
}

impl ToString for AuditAction {
    fn to_string(&self) -> String {
        match self {
            AuditAction::Login => "login".to_string(),
            AuditAction::Logout => "logout".to_string(),
            AuditAction::LoginFailed => "login_failed".to_string(),
            AuditAction::PostCreated => "post_created".to_string(),
            AuditAction::PostUpdated => "post_updated".to_string(),
            AuditAction::PostDeleted => "post_deleted".to_string(),
            AuditAction::PostPublished => "post_published".to_string(),
            AuditAction::PostUnpublished => "post_unpublished".to_string(),
            AuditAction::PortfolioCreated => "portfolio_created".to_string(),
            AuditAction::PortfolioUpdated => "portfolio_updated".to_string(),
            AuditAction::PortfolioDeleted => "portfolio_deleted".to_string(),
            AuditAction::PortfolioFeatured => "portfolio_featured".to_string(),
            AuditAction::PortfolioUnfeatured => "portfolio_unfeatured".to_string(),
            AuditAction::ServiceCreated => "service_created".to_string(),
            AuditAction::ServiceUpdated => "service_updated".to_string(),
            AuditAction::ServiceDeleted => "service_deleted".to_string(),
            AuditAction::ServiceActivated => "service_activated".to_string(),
            AuditAction::ServiceDeactivated => "service_deactivated".to_string(),
            AuditAction::CommentApproved => "comment_approved".to_string(),
            AuditAction::CommentRejected => "comment_rejected".to_string(),
            AuditAction::CommentDeleted => "comment_deleted".to_string(),
            AuditAction::SettingsUpdated => "settings_updated".to_string(),
            AuditAction::ProfileUpdated => "profile_updated".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ResourceType {
    Authentication,
    Post,
    Portfolio,
    Service,
    Comment,
    Settings,
    Profile,
}

impl ToString for ResourceType {
    fn to_string(&self) -> String {
        match self {
            ResourceType::Authentication => "authentication".to_string(),
            ResourceType::Post => "post".to_string(),
            ResourceType::Portfolio => "portfolio".to_string(),
            ResourceType::Service => "service".to_string(),
            ResourceType::Comment => "comment".to_string(),
            ResourceType::Settings => "settings".to_string(),
            ResourceType::Profile => "profile".to_string(),
        }
    }
} 