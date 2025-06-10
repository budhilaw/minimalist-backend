use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::types::ipnetwork::IpNetwork;
use sqlx::FromRow;
use uuid::Uuid;

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

impl std::fmt::Display for AuditAction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            AuditAction::Login => "login",
            AuditAction::Logout => "logout",
            AuditAction::LoginFailed => "login_failed",
            AuditAction::PostCreated => "post_created",
            AuditAction::PostUpdated => "post_updated",
            AuditAction::PostDeleted => "post_deleted",
            AuditAction::PostPublished => "post_published",
            AuditAction::PostUnpublished => "post_unpublished",
            AuditAction::PortfolioCreated => "portfolio_created",
            AuditAction::PortfolioUpdated => "portfolio_updated",
            AuditAction::PortfolioDeleted => "portfolio_deleted",
            AuditAction::PortfolioFeatured => "portfolio_featured",
            AuditAction::PortfolioUnfeatured => "portfolio_unfeatured",
            AuditAction::ServiceCreated => "service_created",
            AuditAction::ServiceUpdated => "service_updated",
            AuditAction::ServiceDeleted => "service_deleted",
            AuditAction::ServiceActivated => "service_activated",
            AuditAction::ServiceDeactivated => "service_deactivated",
            AuditAction::CommentApproved => "comment_approved",
            AuditAction::CommentRejected => "comment_rejected",
            AuditAction::CommentDeleted => "comment_deleted",
            AuditAction::SettingsUpdated => "settings_updated",
            AuditAction::ProfileUpdated => "profile_updated",
        };
        write!(f, "{}", s)
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

impl std::fmt::Display for ResourceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let s = match self {
            ResourceType::Authentication => "authentication",
            ResourceType::Post => "post",
            ResourceType::Portfolio => "portfolio",
            ResourceType::Service => "service",
            ResourceType::Comment => "comment",
            ResourceType::Settings => "settings",
            ResourceType::Profile => "profile",
        };
        write!(f, "{}", s)
    }
}
