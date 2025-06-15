use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::portfolio::{
    CreatePortfolioProjectRequest, PortfolioProject, PortfolioProjectQuery,
    PortfolioProjectsResponse, PortfolioStats, UpdatePortfolioProjectRequest,
};
use crate::utils::errors::AppError;

#[async_trait]
pub trait PortfolioRepositoryTrait: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<PortfolioProject>, AppError>;
    async fn find_by_slug(&self, slug: &str) -> Result<Option<PortfolioProject>, AppError>;
    async fn find_all(
        &self,
        query: PortfolioProjectQuery,
    ) -> Result<PortfolioProjectsResponse, AppError>;
    async fn create(
        &self,
        project: CreatePortfolioProjectRequest,
    ) -> Result<PortfolioProject, AppError>;
    async fn update(
        &self,
        id: Uuid,
        project: UpdatePortfolioProjectRequest,
    ) -> Result<PortfolioProject, AppError>;
    async fn delete(&self, id: Uuid) -> Result<(), AppError>;
    async fn get_featured(&self, limit: Option<u32>) -> Result<Vec<PortfolioProject>, AppError>;
    async fn get_stats(&self) -> Result<PortfolioStats, AppError>;
    async fn update_featured_status(&self, id: Uuid, featured: bool) -> Result<(), AppError>;
}

pub struct PortfolioRepository {
    pool: PgPool,
}

impl PortfolioRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl PortfolioRepositoryTrait for PortfolioRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<PortfolioProject>, AppError> {
        let project = sqlx::query_as::<_, PortfolioProject>(
            r#"
            SELECT id, title, slug, description, long_description, category, technologies, 
                   live_url, github_url, image_url, featured, active, status, start_date, 
                   end_date, client, created_at, updated_at
            FROM portfolio_projects 
            WHERE id = $1
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch portfolio project by id")?;

        Ok(project)
    }

    async fn find_by_slug(&self, slug: &str) -> Result<Option<PortfolioProject>, AppError> {
        let project = sqlx::query_as::<_, PortfolioProject>(
            r#"
            SELECT id, title, slug, description, long_description, category, technologies, 
                   live_url, github_url, image_url, featured, active, status, start_date, 
                   end_date, client, created_at, updated_at
            FROM portfolio_projects 
            WHERE slug = $1
            "#,
        )
        .bind(slug)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch portfolio project by slug")?;

        Ok(project)
    }

    async fn find_all(
        &self,
        query: PortfolioProjectQuery,
    ) -> Result<PortfolioProjectsResponse, AppError> {
        let limit = query.limit.unwrap_or(10).min(100);
        let offset = (query.page.unwrap_or(1) - 1) * limit;

        // Build WHERE clause - if active is not specified, return ALL projects (for admin)
        let (where_clause, count_query, projects_query) = if let Some(active) = query.active {
            // Filter by active status
            let where_clause = "WHERE active = $1";
            let count_query = format!("SELECT COUNT(*) FROM portfolio_projects {}", where_clause);
            let projects_query = format!(
                r#"
                SELECT id, title, slug, description, long_description, category, technologies, 
                       live_url, github_url, image_url, featured, active, status, start_date, 
                       end_date, client, created_at, updated_at
                FROM portfolio_projects 
                {}
                ORDER BY featured DESC, created_at DESC 
                LIMIT $2 OFFSET $3
                "#,
                where_clause
            );
            (Some(active), count_query, projects_query)
        } else {
            // Return ALL projects (admin view)
            let count_query = "SELECT COUNT(*) FROM portfolio_projects".to_string();
            let projects_query = r#"
                SELECT id, title, slug, description, long_description, category, technologies, 
                       live_url, github_url, image_url, featured, active, status, start_date, 
                       end_date, client, created_at, updated_at
                FROM portfolio_projects 
                ORDER BY featured DESC, created_at DESC 
                LIMIT $1 OFFSET $2
            "#
            .to_string();
            (None, count_query, projects_query)
        };

        // Get total count
        let total: i64 = if let Some(active) = where_clause {
            sqlx::query_scalar(&count_query)
                .bind(active)
                .fetch_one(&self.pool)
                .await
                .context("Failed to count portfolio projects")?
        } else {
            sqlx::query_scalar(&count_query)
                .fetch_one(&self.pool)
                .await
                .context("Failed to count portfolio projects")?
        };

        // Get projects
        let projects = if let Some(active) = where_clause {
            sqlx::query_as::<_, PortfolioProject>(&projects_query)
                .bind(active)
                .bind(limit as i64)
                .bind(offset as i64)
                .fetch_all(&self.pool)
                .await
                .context("Failed to fetch portfolio projects")?
        } else {
            sqlx::query_as::<_, PortfolioProject>(&projects_query)
                .bind(limit as i64)
                .bind(offset as i64)
                .fetch_all(&self.pool)
                .await
                .context("Failed to fetch portfolio projects")?
        };

        let total_pages = (total as f64 / limit as f64).ceil() as u32;

        Ok(PortfolioProjectsResponse {
            projects: projects.into_iter().map(|p| p.into()).collect(),
            total,
            page: query.page.unwrap_or(1),
            limit,
            total_pages,
        })
    }

    async fn create(
        &self,
        project: CreatePortfolioProjectRequest,
    ) -> Result<PortfolioProject, AppError> {
        let created_project = sqlx::query_as::<_, PortfolioProject>(
            r#"
            INSERT INTO portfolio_projects (
                title, slug, description, long_description, category, technologies, 
                live_url, github_url, image_url, featured, active, status, start_date, 
                end_date, client
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            RETURNING id, title, slug, description, long_description, category, technologies, 
                      live_url, github_url, image_url, featured, active, status, start_date, 
                      end_date, client, created_at, updated_at
            "#,
        )
        .bind(&project.title)
        .bind(&project.slug)
        .bind(&project.description)
        .bind(&project.long_description)
        .bind(&project.category)
        .bind(&project.technologies)
        .bind(&project.live_url)
        .bind(&project.github_url)
        .bind(&project.image_url)
        .bind(project.featured.unwrap_or(false))
        .bind(project.active.unwrap_or(true))
        .bind(&project.status)
        .bind(project.start_date)
        .bind(project.end_date)
        .bind(&project.client)
        .fetch_one(&self.pool)
        .await
        .context("Failed to create portfolio project")?;

        Ok(created_project)
    }

    async fn update(
        &self,
        id: Uuid,
        project: UpdatePortfolioProjectRequest,
    ) -> Result<PortfolioProject, AppError> {
        let updated_project = sqlx::query_as::<_, PortfolioProject>(
            r#"
            UPDATE portfolio_projects 
            SET title = $1, slug = $2, description = $3, long_description = $4, category = $5, 
                technologies = $6, live_url = $7, github_url = $8, image_url = $9, 
                featured = $10, active = $11, status = $12, start_date = $13, end_date = $14, 
                client = $15, updated_at = NOW()
            WHERE id = $16
            RETURNING id, title, slug, description, long_description, category, technologies, 
                      live_url, github_url, image_url, featured, active, status, start_date, 
                      end_date, client, created_at, updated_at
            "#,
        )
        .bind(&project.title)
        .bind(&project.slug)
        .bind(&project.description)
        .bind(&project.long_description)
        .bind(&project.category)
        .bind(&project.technologies)
        .bind(&project.live_url)
        .bind(&project.github_url)
        .bind(&project.image_url)
        .bind(project.featured.unwrap_or(false))
        .bind(project.active.unwrap_or(true))
        .bind(&project.status)
        .bind(project.start_date)
        .bind(project.end_date)
        .bind(&project.client)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to update portfolio project")?
        .ok_or(AppError::NotFound(
            "Portfolio project not found".to_string(),
        ))?;

        Ok(updated_project)
    }

    async fn delete(&self, id: Uuid) -> Result<(), AppError> {
        let result = sqlx::query("DELETE FROM portfolio_projects WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .context("Failed to delete portfolio project")?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(
                "Portfolio project not found".to_string(),
            ));
        }

        Ok(())
    }

    async fn get_featured(&self, limit: Option<u32>) -> Result<Vec<PortfolioProject>, AppError> {
        let limit = limit.unwrap_or(6).min(20);

        let projects = sqlx::query_as::<_, PortfolioProject>(
            r#"
            SELECT id, title, slug, description, long_description, category, technologies, 
                   live_url, github_url, image_url, featured, active, status, start_date, 
                   end_date, client, created_at, updated_at
            FROM portfolio_projects 
            WHERE featured = true AND active = true
            ORDER BY created_at DESC 
            LIMIT $1
            "#,
        )
        .bind(limit as i64)
        .fetch_all(&self.pool)
        .await
        .context("Failed to fetch featured portfolio projects")?;

        Ok(projects)
    }

    async fn get_stats(&self) -> Result<PortfolioStats, AppError> {
        let total_projects: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM portfolio_projects")
            .fetch_one(&self.pool)
            .await
            .context("Failed to count total projects")?;

        let completed_projects: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM portfolio_projects WHERE status = 'completed'",
        )
        .fetch_one(&self.pool)
        .await
        .context("Failed to count completed projects")?;

        let in_progress_projects: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM portfolio_projects WHERE status = 'in_progress'",
        )
        .fetch_one(&self.pool)
        .await
        .context("Failed to count in progress projects")?;

        let featured_projects: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM portfolio_projects WHERE featured = true")
                .fetch_one(&self.pool)
                .await
                .context("Failed to count featured projects")?;

        let projects_this_year: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM portfolio_projects WHERE EXTRACT(YEAR FROM created_at) = EXTRACT(YEAR FROM CURRENT_DATE)"
        )
        .fetch_one(&self.pool)
        .await
        .context("Failed to count projects this year")?;

        Ok(PortfolioStats {
            total_projects,
            completed_projects,
            in_progress_projects,
            featured_projects,
            projects_this_year,
        })
    }

    async fn update_featured_status(&self, id: Uuid, featured: bool) -> Result<(), AppError> {
        let result = sqlx::query(
            "UPDATE portfolio_projects SET featured = $1, updated_at = NOW() WHERE id = $2",
        )
        .bind(featured)
        .bind(id)
        .execute(&self.pool)
        .await
        .context("Failed to update featured status")?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound(
                "Portfolio project not found".to_string(),
            ));
        }

        Ok(())
    }
}
