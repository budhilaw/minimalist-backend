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
    models::service::{CreateServiceRequest, ServiceQuery, UpdateServiceRequest},
    services::service_service::ServiceServiceTrait,
    utils::errors::AppError,
};

#[derive(Clone)]
pub struct ServiceState {
    pub service_service: Arc<dyn ServiceServiceTrait>,
}

// GET /api/v1/services
pub async fn get_all_services(
    State(state): State<ServiceState>,
    Query(query): Query<ServiceQuery>,
) -> Result<Json<Value>, AppError> {
    let response = state.service_service.get_all_services(query).await?;
    Ok(Json(json!(response)))
}

// GET /api/v1/services/:id
pub async fn get_service(
    State(state): State<ServiceState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, AppError> {
    let service = state
        .service_service
        .get_service_by_id(id)
        .await?
        .ok_or_else(|| AppError::NotFound("Service not found".to_string()))?;

    Ok(Json(json!(service)))
}

// POST /api/v1/services
pub async fn create_service(
    State(state): State<ServiceState>,
    Json(payload): Json<CreateServiceRequest>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    // Validate the request
    payload
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let service = state.service_service.create_service(payload).await?;

    Ok((
        StatusCode::CREATED,
        Json(json!({
            "message": "Service created successfully",
            "service": service
        })),
    ))
}

// PUT /api/v1/services/:id
pub async fn update_service(
    State(state): State<ServiceState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<UpdateServiceRequest>,
) -> Result<Json<Value>, AppError> {
    // Validate the request
    payload
        .validate()
        .map_err(|e| AppError::Validation(e.to_string()))?;

    let service = state.service_service.update_service(id, payload).await?;

    Ok(Json(json!({
        "message": "Service updated successfully",
        "service": service
    })))
}

// DELETE /api/v1/services/:id
pub async fn delete_service(
    State(state): State<ServiceState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, AppError> {
    state.service_service.delete_service(id).await?;

    Ok(Json(json!({
        "message": "Service deleted successfully"
    })))
}

// GET /api/v1/services/active
pub async fn get_active_services(
    State(state): State<ServiceState>,
) -> Result<Json<Value>, AppError> {
    let services = state.service_service.get_active_services().await?;

    Ok(Json(json!({
        "services": services,
        "total": services.len()
    })))
}

// GET /api/v1/services/stats
pub async fn get_service_stats(State(state): State<ServiceState>) -> Result<Json<Value>, AppError> {
    let stats = state.service_service.get_service_statistics().await?;
    Ok(Json(json!(stats)))
}

// PUT /api/v1/services/:id/status
pub async fn update_service_status(
    State(state): State<ServiceState>,
    Path(id): Path<Uuid>,
    Json(payload): Json<Value>,
) -> Result<Json<Value>, AppError> {
    let active = payload
        .get("active")
        .and_then(|v| v.as_bool())
        .ok_or_else(|| AppError::Validation("Active status is required".to_string()))?;

    state
        .service_service
        .toggle_service_status(id, active)
        .await?;

    Ok(Json(json!({
        "message": "Service status updated successfully"
    })))
}

// GET /api/v1/services/category/:category
pub async fn get_services_by_category(
    State(state): State<ServiceState>,
    Path(category): Path<String>,
) -> Result<Json<Value>, AppError> {
    let services = state
        .service_service
        .get_services_by_category(&category)
        .await?;

    Ok(Json(json!({
        "services": services,
        "category": category,
        "total": services.len()
    })))
}
