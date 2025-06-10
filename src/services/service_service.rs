use crate::utils::errors::AppError;
use std::sync::Arc;
use uuid::Uuid;
type Result<T> = std::result::Result<T, AppError>;

use crate::{
    models::service::{
        CreateServiceRequest, Service, ServiceQuery, ServiceStats, ServicesResponse,
        UpdateServiceRequest,
    },
    repositories::service_repository::ServiceRepositoryTrait,
};

#[async_trait::async_trait]
pub trait ServiceServiceTrait: Send + Sync {
    async fn get_all_services(&self, query: ServiceQuery) -> Result<ServicesResponse>;
    async fn get_service_by_id(&self, id: Uuid) -> Result<Option<Service>>;
    async fn create_service(&self, request: CreateServiceRequest) -> Result<Service>;
    async fn update_service(&self, id: Uuid, request: UpdateServiceRequest) -> Result<Service>;
    async fn delete_service(&self, id: Uuid) -> Result<()>;
    async fn get_active_services(&self) -> Result<Vec<Service>>;
    async fn get_service_statistics(&self) -> Result<ServiceStats>;
    async fn toggle_service_status(&self, id: Uuid, active: bool) -> Result<()>;
    async fn get_services_by_category(&self, category: &str) -> Result<Vec<Service>>;
}

#[derive(Clone)]
pub struct ServiceService {
    repository: Arc<dyn ServiceRepositoryTrait>,
}

impl ServiceService {
    pub fn new(repository: Arc<dyn ServiceRepositoryTrait>) -> Self {
        Self { repository }
    }
}

#[async_trait::async_trait]
impl ServiceServiceTrait for ServiceService {
    async fn get_all_services(&self, query: ServiceQuery) -> Result<ServicesResponse> {
        // Business logic: Apply default pagination
        let query = ServiceQuery {
            page: query.page.or(Some(1)),
            limit: query.limit.or(Some(10)),
            ..query
        };

        self.repository.find_all(query).await
    }

    async fn get_service_by_id(&self, id: Uuid) -> Result<Option<Service>> {
        self.repository.find_by_id(id).await
    }

    async fn create_service(&self, request: CreateServiceRequest) -> Result<Service> {
        // Business logic: Validate service data
        self.validate_service_request(&request.title, &request.description)?;

        // Business logic: Normalize category
        let mut request = request;
        request.category = self.normalize_category(&request.category);

        self.repository.create(request).await
    }

    async fn update_service(&self, id: Uuid, request: UpdateServiceRequest) -> Result<Service> {
        // Business logic: Ensure service exists
        if self.repository.find_by_id(id).await?.is_none() {
            return Err(AppError::NotFound("Service not found".to_string()));
        }

        // Business logic: Validate service data
        self.validate_service_request(&request.title, &request.description)?;

        // Business logic: Normalize category
        let mut request = request;
        request.category = self.normalize_category(&request.category);

        self.repository.update(id, request).await
    }

    async fn delete_service(&self, id: Uuid) -> Result<()> {
        // Business logic: Ensure service exists
        if self.repository.find_by_id(id).await?.is_none() {
            return Err(AppError::NotFound("Service not found".to_string()));
        }

        // Business logic: Check if service can be deleted (no active bookings, etc.)
        // In a real application, you might check for dependencies here

        self.repository.delete(id).await
    }

    async fn get_active_services(&self) -> Result<Vec<Service>> {
        self.repository.get_active().await
    }

    async fn get_service_statistics(&self) -> Result<ServiceStats> {
        self.repository.get_stats().await
    }

    async fn toggle_service_status(&self, id: Uuid, active: bool) -> Result<()> {
        // Business logic: Ensure service exists
        if self.repository.find_by_id(id).await?.is_none() {
            return Err(AppError::NotFound("Service not found".to_string()));
        }

        self.repository.update_active_status(id, active).await
    }

    async fn get_services_by_category(&self, category: &str) -> Result<Vec<Service>> {
        // Business logic: Normalize category for search
        let normalized_category = self.normalize_category(category);
        self.repository.get_by_category(&normalized_category).await
    }
}

impl ServiceService {
    fn validate_service_request(&self, title: &str, description: &str) -> Result<()> {
        if title.trim().is_empty() {
            return Err(AppError::Validation(
                "Service title cannot be empty".to_string(),
            ));
        }

        if title.trim().len() < 3 {
            return Err(AppError::Validation(
                "Service title must be at least 3 characters long".to_string(),
            ));
        }

        if description.trim().is_empty() {
            return Err(AppError::Validation(
                "Service description cannot be empty".to_string(),
            ));
        }

        if description.trim().len() < 10 {
            return Err(AppError::Validation(
                "Service description must be at least 10 characters long".to_string(),
            ));
        }

        Ok(())
    }

    fn normalize_category(&self, category: &str) -> String {
        category
            .trim()
            .to_lowercase()
            .chars()
            .map(|c| {
                if c.is_alphanumeric() || c == ' ' {
                    c
                } else {
                    ' '
                }
            })
            .collect::<String>()
            .split_whitespace()
            .collect::<Vec<&str>>()
            .join(" ")
    }
}
