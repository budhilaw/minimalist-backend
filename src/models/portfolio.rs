use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use validator::Validate;

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct PortfolioProject {
    pub id: Uuid,
    pub title: String,
    pub slug: String,
    pub description: String,
    pub long_description: Option<String>,
    pub category: String,
    pub technologies: Vec<String>,
    pub live_url: Option<String>,
    pub github_url: Option<String>,
    pub image_url: Option<String>,
    pub featured: bool,
    pub active: bool,
    pub status: String,
    pub start_date: NaiveDate,
    pub end_date: Option<NaiveDate>,
    pub client: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct PortfolioProjectResponse {
    pub id: Uuid,
    pub title: String,
    pub slug: String,
    pub description: String,
    pub long_description: Option<String>,
    pub category: String,
    pub technologies: Vec<String>,
    pub live_url: Option<String>,
    pub github_url: Option<String>,
    pub image_url: Option<String>,
    pub featured: bool,
    pub active: bool,
    pub status: String,
    pub start_date: NaiveDate,
    pub end_date: Option<NaiveDate>,
    pub client: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<PortfolioProject> for PortfolioProjectResponse {
    fn from(project: PortfolioProject) -> Self {
        Self {
            id: project.id,
            title: project.title,
            slug: project.slug,
            description: project.description,
            long_description: project.long_description,
            category: project.category,
            technologies: project.technologies,
            live_url: project.live_url,
            github_url: project.github_url,
            image_url: project.image_url,
            featured: project.featured,
            active: project.active,
            status: project.status,
            start_date: project.start_date,
            end_date: project.end_date,
            client: project.client,
            created_at: project.created_at,
            updated_at: project.updated_at,
        }
    }
}

#[derive(Debug, Deserialize, Validate)]
pub struct CreatePortfolioProjectRequest {
    #[validate(length(
        min = 1,
        max = 255,
        message = "Title is required and must be less than 255 characters"
    ))]
    pub title: String,
    #[validate(length(
        min = 1,
        max = 255,
        message = "Slug is required and must be less than 255 characters"
    ))]
    pub slug: String,
    #[validate(length(min = 1, message = "Description is required"))]
    pub description: String,
    pub long_description: Option<String>,
    #[validate(length(
        min = 1,
        max = 50,
        message = "Category is required and must be less than 50 characters"
    ))]
    pub category: String,
    pub technologies: Vec<String>,
    #[validate(url(message = "Live URL must be a valid URL"))]
    pub live_url: Option<String>,
    #[validate(url(message = "GitHub URL must be a valid URL"))]
    pub github_url: Option<String>,
    #[validate(url(message = "Image URL must be a valid URL"))]
    pub image_url: Option<String>,
    pub featured: Option<bool>,
    pub active: Option<bool>,
    #[validate(length(
        min = 1,
        max = 20,
        message = "Status is required and must be less than 20 characters"
    ))]
    pub status: String,
    pub start_date: NaiveDate,
    pub end_date: Option<NaiveDate>,
    #[validate(length(max = 255, message = "Client name must be less than 255 characters"))]
    pub client: Option<String>,
}

#[derive(Debug, Deserialize, Validate)]
pub struct UpdatePortfolioProjectRequest {
    #[validate(length(
        min = 1,
        max = 255,
        message = "Title is required and must be less than 255 characters"
    ))]
    pub title: String,
    #[validate(length(
        min = 1,
        max = 255,
        message = "Slug is required and must be less than 255 characters"
    ))]
    pub slug: String,
    #[validate(length(min = 1, message = "Description is required"))]
    pub description: String,
    pub long_description: Option<String>,
    #[validate(length(
        min = 1,
        max = 50,
        message = "Category is required and must be less than 50 characters"
    ))]
    pub category: String,
    pub technologies: Vec<String>,
    #[validate(url(message = "Live URL must be a valid URL"))]
    pub live_url: Option<String>,
    #[validate(url(message = "GitHub URL must be a valid URL"))]
    pub github_url: Option<String>,
    #[validate(url(message = "Image URL must be a valid URL"))]
    pub image_url: Option<String>,
    pub featured: Option<bool>,
    pub active: Option<bool>,
    #[validate(length(
        min = 1,
        max = 20,
        message = "Status is required and must be less than 20 characters"
    ))]
    pub status: String,
    pub start_date: NaiveDate,
    pub end_date: Option<NaiveDate>,
    #[validate(length(max = 255, message = "Client name must be less than 255 characters"))]
    pub client: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct PortfolioProjectQuery {
    pub page: Option<u32>,
    pub limit: Option<u32>,
    pub category: Option<String>,
    pub status: Option<String>,
    pub featured: Option<bool>,
    pub active: Option<bool>,
    pub technologies: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct PortfolioProjectsResponse {
    pub projects: Vec<PortfolioProjectResponse>,
    pub total: i64,
    pub page: u32,
    pub limit: u32,
    pub total_pages: u32,
}

#[derive(Debug, Serialize)]
pub struct PortfolioStats {
    pub total_projects: i64,
    pub completed_projects: i64,
    pub in_progress_projects: i64,
    pub featured_projects: i64,
    pub projects_this_year: i64,
}
