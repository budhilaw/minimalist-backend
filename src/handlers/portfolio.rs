use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;
use validator::Validate;

use crate::{
    models::portfolio::{
        CreatePortfolioProjectRequest, PortfolioProjectQuery, UpdatePortfolioProjectRequest,
    },
    services::portfolio_service::PortfolioServiceTrait,
    utils::errors::AppError,
};

#[derive(Clone)]
pub struct PortfolioState {
    pub portfolio_service: Arc<dyn PortfolioServiceTrait>,
}

// GET /api/v1/portfolio
pub async fn get_all_projects(
    State(state): State<PortfolioState>,
    Query(query): Query<PortfolioProjectQuery>,
) -> Result<Json<Value>, AppError> {
    let response = state.portfolio_service.get_all_projects(query).await?;
    Ok(Json(json!(response)))
}

// GET /api/v1/portfolio/:id
pub async fn get_project(
    State(state): State<PortfolioState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, AppError> {
    let project = state
        .portfolio_service
        .get_project_by_id(id)
        .await?
        .ok_or_else(|| AppError::NotFound("Portfolio project not found".to_string()))?;

    Ok(Json(json!(project)))
}

// POST /api/v1/portfolio
pub async fn create_project(
    State(state): State<PortfolioState>,
    Json(payload): Json<CreatePortfolioProjectRequest>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    // Validate the request
    payload
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let project = state.portfolio_service.create_project(payload).await?;

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "message": "Portfolio project created successfully",
            "project": project
        })),
    ))
}

// PUT /api/v1/portfolio/:id
pub async fn update_project(
    State(state): State<PortfolioState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdatePortfolioProjectRequest>,
) -> Result<Json<Value>, AppError> {
    // Validate the request
    payload
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let project = state.portfolio_service.update_project(id, payload).await?;

    Ok(Json(json!({
        "message": "Portfolio project updated successfully",
        "project": project
    })))
}

// DELETE /api/v1/portfolio/:id
pub async fn delete_project(
    State(state): State<PortfolioState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, AppError> {
    state.portfolio_service.delete_project(id).await?;

    Ok(Json(json!({
        "message": "Portfolio project deleted successfully"
    })))
}

// GET /api/v1/portfolio/featured
pub async fn get_featured_projects(
    State(state): State<PortfolioState>,
    Query(query): Query<serde_json::Value>,
) -> Result<Json<Value>, AppError> {
    let limit = query
        .get("limit")
        .and_then(|v| v.as_u64())
        .map(|v| v as u32);

    let projects = state.portfolio_service.get_featured_projects(limit).await?;

    Ok(Json(json!({
        "projects": projects,
        "total": projects.len()
    })))
}

// GET /api/v1/portfolio/stats
pub async fn get_portfolio_stats(
    State(state): State<PortfolioState>,
) -> Result<Json<Value>, AppError> {
    let stats = state.portfolio_service.get_portfolio_statistics().await?;
    Ok(Json(json!(stats)))
}

// PUT /api/v1/portfolio/:id/featured
pub async fn update_featured_status(
    State(state): State<PortfolioState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, AppError> {
    let featured = payload
        .get("featured")
        .and_then(|v| v.as_bool())
        .ok_or_else(|| AppError::Validation("Featured status is required".to_string()))?;

    state
        .portfolio_service
        .toggle_featured_status(id, featured)
        .await?;

    Ok(Json(json!({
        "message": "Featured status updated successfully"
    })))
}
