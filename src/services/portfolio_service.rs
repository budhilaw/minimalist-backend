use crate::utils::errors::AppError;
use std::sync::Arc;
use uuid::Uuid;
type Result<T> = std::result::Result<T, AppError>;

use crate::{
    models::portfolio::{
        CreatePortfolioProjectRequest, PortfolioProject, PortfolioProjectQuery,
        PortfolioProjectsResponse, PortfolioStats, UpdatePortfolioProjectRequest,
    },
    repositories::portfolio_repository::PortfolioRepositoryTrait,
};

#[async_trait::async_trait]
pub trait PortfolioServiceTrait: Send + Sync {
    async fn get_all_projects(
        &self,
        query: PortfolioProjectQuery,
    ) -> Result<PortfolioProjectsResponse>;
    async fn get_project_by_id(&self, id: Uuid) -> Result<Option<PortfolioProject>>;
    async fn create_project(
        &self,
        request: CreatePortfolioProjectRequest,
    ) -> Result<PortfolioProject>;
    async fn update_project(
        &self,
        id: Uuid,
        request: UpdatePortfolioProjectRequest,
    ) -> Result<PortfolioProject>;
    async fn delete_project(&self, id: Uuid) -> Result<()>;
    async fn get_featured_projects(&self, limit: Option<u32>) -> Result<Vec<PortfolioProject>>;
    async fn get_portfolio_statistics(&self) -> Result<PortfolioStats>;
    async fn toggle_featured_status(&self, id: Uuid, featured: bool) -> Result<()>;
}

#[derive(Clone)]
pub struct PortfolioService {
    repository: Arc<dyn PortfolioRepositoryTrait>,
}

impl PortfolioService {
    pub fn new(repository: Arc<dyn PortfolioRepositoryTrait>) -> Self {
        Self { repository }
    }
}

#[async_trait::async_trait]
impl PortfolioServiceTrait for PortfolioService {
    async fn get_all_projects(
        &self,
        query: PortfolioProjectQuery,
    ) -> Result<PortfolioProjectsResponse> {
        // Business logic: Apply default pagination if not specified
        let query = PortfolioProjectQuery {
            page: query.page.or(Some(1)),
            limit: query.limit.or(Some(10)),
            ..query
        };

        self.repository.find_all(query).await
    }

    async fn get_project_by_id(&self, id: Uuid) -> Result<Option<PortfolioProject>> {
        self.repository.find_by_id(id).await
    }

    async fn create_project(
        &self,
        request: CreatePortfolioProjectRequest,
    ) -> Result<PortfolioProject> {
        // Business logic: Validate business rules
        if request.title.trim().is_empty() {
            return Err(AppError::Validation(
                "Project title cannot be empty".to_string(),
            ));
        }

        if request.description.trim().is_empty() {
            return Err(AppError::Validation(
                "Project description cannot be empty".to_string(),
            ));
        }

        // Business logic: Portfolio projects don't use slugs in this model
        // This validation was for a different model structure

        self.repository.create(request).await
    }

    async fn update_project(
        &self,
        id: Uuid,
        request: UpdatePortfolioProjectRequest,
    ) -> Result<PortfolioProject> {
        // Business logic: Ensure project exists before updating
        if self.repository.find_by_id(id).await?.is_none() {
            return Err(AppError::NotFound(
                "Portfolio project not found".to_string(),
            ));
        }

        // Business logic: Validate business rules
        if request.title.trim().is_empty() {
            return Err(AppError::Validation(
                "Project title cannot be empty".to_string(),
            ));
        }

        if request.description.trim().is_empty() {
            return Err(AppError::Validation(
                "Project description cannot be empty".to_string(),
            ));
        }

        self.repository.update(id, request).await
    }

    async fn delete_project(&self, id: Uuid) -> Result<()> {
        // Business logic: Ensure project exists before deleting
        if self.repository.find_by_id(id).await?.is_none() {
            return Err(AppError::NotFound(
                "Portfolio project not found".to_string(),
            ));
        }

        self.repository.delete(id).await
    }

    async fn get_featured_projects(&self, limit: Option<u32>) -> Result<Vec<PortfolioProject>> {
        // Business logic: Apply reasonable default limit
        let limit = limit.unwrap_or(5);

        // Business logic: Don't allow excessive requests
        if limit > 50 {
            return Err(AppError::Validation(
                "Limit cannot exceed 50 projects".to_string(),
            ));
        }

        self.repository.get_featured(Some(limit)).await
    }

    async fn get_portfolio_statistics(&self) -> Result<PortfolioStats> {
        self.repository.get_stats().await
    }

    async fn toggle_featured_status(&self, id: Uuid, featured: bool) -> Result<()> {
        // Business logic: Ensure project exists
        if self.repository.find_by_id(id).await?.is_none() {
            return Err(AppError::NotFound(
                "Portfolio project not found".to_string(),
            ));
        }

        // Business logic: Limit number of featured projects
        if featured {
            let stats = self.repository.get_stats().await?;
            if stats.featured_projects >= 10 {
                return Err(AppError::Validation(
                    "Cannot have more than 10 featured projects".to_string(),
                ));
            }
        }

        self.repository.update_featured_status(id, featured).await
    }
}

impl PortfolioService {}
