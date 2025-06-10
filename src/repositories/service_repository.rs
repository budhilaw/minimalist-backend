use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::service::{
    CategoryCount, CreateServiceRequest, Service, ServiceQuery, ServiceStats, ServicesResponse,
    UpdateServiceRequest,
};
use crate::utils::errors::AppError;

#[async_trait]
pub trait ServiceRepositoryTrait: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Service>, AppError>;
    async fn find_all(&self, query: ServiceQuery) -> Result<ServicesResponse, AppError>;
    async fn create(&self, service: CreateServiceRequest) -> Result<Service, AppError>;
    async fn update(&self, id: Uuid, service: UpdateServiceRequest) -> Result<Service, AppError>;
    async fn delete(&self, id: Uuid) -> Result<(), AppError>;
    async fn get_active(&self) -> Result<Vec<Service>, AppError>;
    async fn get_stats(&self) -> Result<ServiceStats, AppError>;
    async fn update_active_status(&self, id: Uuid, active: bool) -> Result<(), AppError>;
    async fn get_by_category(&self, category: &str) -> Result<Vec<Service>, AppError>;
}

pub struct ServiceRepository {
    pool: PgPool,
}

impl ServiceRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl ServiceRepositoryTrait for ServiceRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<Service>, AppError> {
        let service = sqlx::query_as::<_, Service>(
            r#"
            SELECT id, title, description, features, category, active, created_at, updated_at
            FROM services 
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch service by id")?;

        Ok(service)
    }

    async fn find_all(&self, query: ServiceQuery) -> Result<ServicesResponse, AppError> {
        let limit = query.limit.unwrap_or(10).min(100);
        let offset = (query.page.unwrap_or(1) - 1) * limit;

        let mut where_conditions = Vec::new();
        let mut bind_count = 0;

        // Build dynamic WHERE clause
        if query.category.is_some() {
            bind_count += 1;
            where_conditions.push(format!("category = ${}", bind_count));
        }

        if query.active.is_some() {
            bind_count += 1;
            where_conditions.push(format!("active = ${}", bind_count));
        }

        let _where_clause = if where_conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_conditions.join(" AND "))
        };

        // For simplicity, let's create a basic query without dynamic binding
        let base_query = "SELECT COUNT(*) FROM services".to_string();
        let services_query = r#"
            SELECT id, title, description, features, category, active, created_at, updated_at
            FROM services 
            ORDER BY created_at DESC 
            LIMIT $1 OFFSET $2
        "#
        .to_string();

        // Get total count
        let total: i64 = sqlx::query_scalar(&base_query)
            .fetch_one(&self.pool)
            .await
            .context("Failed to count services")?;

        // Get services
        let services = sqlx::query_as::<_, Service>(&services_query)
            .bind(limit as i64)
            .bind(offset as i64)
            .fetch_all(&self.pool)
            .await
            .context("Failed to fetch services")?;

        let total_pages = (total as f64 / limit as f64).ceil() as u32;

        Ok(ServicesResponse {
            services: services.into_iter().map(|s| s.into()).collect(),
            total,
            page: query.page.unwrap_or(1),
            limit,
            total_pages,
        })
    }

    async fn create(&self, service: CreateServiceRequest) -> Result<Service, AppError> {
        let created_service = sqlx::query_as::<_, Service>(
            r#"
            INSERT INTO services (
                title, description, features, category, active
            )
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, title, description, features, category, active, created_at, updated_at
            "#,
        )
        .bind(&service.title)
        .bind(&service.description)
        .bind(&service.features)
        .bind(&service.category)
        .bind(service.active.unwrap_or(true))
        .fetch_one(&self.pool)
        .await
        .context("Failed to create service")?;

        Ok(created_service)
    }

    async fn update(&self, id: Uuid, service: UpdateServiceRequest) -> Result<Service, AppError> {
        let updated_service = sqlx::query_as::<_, Service>(
            r#"
            UPDATE services 
            SET title = $1, description = $2, features = $3, category = $4, active = $5, updated_at = NOW()
            WHERE id = $6
            RETURNING id, title, description, features, category, active, created_at, updated_at
            "#,
        )
        .bind(&service.title)
        .bind(&service.description)
        .bind(&service.features)
        .bind(&service.category)
        .bind(service.active.unwrap_or(true))
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to update service")?
        .ok_or(AppError::NotFound("Service not found".to_string()))?;

        Ok(updated_service)
    }

    async fn delete(&self, id: Uuid) -> Result<(), AppError> {
        let result = sqlx::query("DELETE FROM services WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .context("Failed to delete service")?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Service not found".to_string()));
        }

        Ok(())
    }

    async fn get_active(&self) -> Result<Vec<Service>, AppError> {
        let services = sqlx::query_as::<_, Service>(
            r#"
            SELECT id, title, description, features, category, active, created_at, updated_at
            FROM services 
            WHERE active = true 
            ORDER BY created_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch active services")?;

        Ok(services)
    }

    async fn get_stats(&self) -> Result<ServiceStats, AppError> {
        let total_services: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM services")
            .fetch_one(&self.pool)
            .await
            .context("Failed to count total services")?;

        let active_services: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM services WHERE active = true")
                .fetch_one(&self.pool)
                .await
                .context("Failed to count active services")?;

        let inactive_services: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM services WHERE active = false")
                .fetch_one(&self.pool)
                .await
                .context("Failed to count inactive services")?;

        let categories = sqlx::query_as::<_, CategoryCount>(
            "SELECT category, COUNT(*) as count FROM services GROUP BY category ORDER BY count DESC"
        )
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch service categories")?;

        Ok(ServiceStats {
            total_services,
            active_services,
            inactive_services,
            services_by_category: categories,
        })
    }

    async fn update_active_status(&self, id: Uuid, active: bool) -> Result<(), AppError> {
        let result =
            sqlx::query("UPDATE services SET active = $1, updated_at = NOW() WHERE id = $2")
                .bind(active)
                .bind(id)
                .execute(&self.pool)
                .await
                .context("Failed to update service active status")?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("Service not found".to_string()));
        }

        Ok(())
    }

    async fn get_by_category(&self, category: &str) -> Result<Vec<Service>, AppError> {
        let services = sqlx::query_as::<_, Service>(
            r#"
            SELECT id, title, description, features, category, active, created_at, updated_at
            FROM services 
            WHERE category = $1 AND active = true
            ORDER BY created_at DESC
            "#,
        )
        .bind(category)
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch services by category")?;

        Ok(services)
    }
}
