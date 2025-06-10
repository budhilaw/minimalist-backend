use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct Service {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub features: Vec<String>,
    pub category: String,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ServiceResponse {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub features: Vec<String>,
    pub category: String,
    pub active: bool,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<Service> for ServiceResponse {
    fn from(service: Service) -> Self {
        Self {
            id: service.id,
            title: service.title,
            description: service.description,
            features: service.features,
            category: service.category,
            active: service.active,
            created_at: service.created_at,
            updated_at: service.updated_at,
        }
    }
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreateServiceRequest {
    #[validate(length(
        min = 1,
        max = 255,
        message = "Title is required and must be less than 255 characters"
    ))]
    pub title: String,
    #[validate(length(min = 1, message = "Description is required"))]
    pub description: String,
    pub features: Vec<String>,
    #[validate(length(
        min = 1,
        max = 100,
        message = "Category is required and must be less than 100 characters"
    ))]
    pub category: String,
    pub active: Option<bool>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdateServiceRequest {
    #[validate(length(
        min = 1,
        max = 255,
        message = "Title is required and must be less than 255 characters"
    ))]
    pub title: String,
    #[validate(length(min = 1, message = "Description is required"))]
    pub description: String,
    pub features: Vec<String>,
    #[validate(length(
        min = 1,
        max = 100,
        message = "Category is required and must be less than 100 characters"
    ))]
    pub category: String,
    pub active: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct ServiceQuery {
    pub page: Option<u32>,
    pub limit: Option<u32>,
    pub category: Option<String>,
    pub active: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct ServicesResponse {
    pub services: Vec<ServiceResponse>,
    pub total: i64,
    pub page: u32,
    pub limit: u32,
    pub total_pages: u32,
}

#[derive(Debug, Serialize)]
pub struct ServiceStats {
    pub total_services: i64,
    pub active_services: i64,
    pub inactive_services: i64,
    pub services_by_category: Vec<CategoryCount>,
}

#[derive(Debug, Serialize, FromRow)]
pub struct CategoryCount {
    pub category: String,
    pub count: i64,
}
