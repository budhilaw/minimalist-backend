use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::Json,
};
use serde_json::{json, Value};
use std::sync::Arc;
use tracing::info;
use uuid::Uuid;

use crate::{
    models::audit_log::{AuditLogFilters, CreateAuditLogRequest},
    services::audit_log_service::AuditLogServiceTrait,
    utils::errors::AppError,
};

#[derive(Clone)]
pub struct AuditLogState {
    pub audit_log_service: Arc<dyn AuditLogServiceTrait>,
}

// GET /api/v1/admin/audit-logs
pub async fn get_audit_logs(
    State(state): State<AuditLogState>,
    Query(filters): Query<AuditLogFilters>,
) -> Result<Json<Value>, AppError> {
    info!(
        "get_audit_logs: Starting request with filters: {:?}",
        filters
    );

    let response = state
        .audit_log_service
        .get_all_with_filters(filters)
        .await?;

    info!(
        "get_audit_logs: Successfully fetched {} logs",
        response.logs.len()
    );
    Ok(Json(json!(response)))
}

// GET /api/v1/admin/audit-logs/:id
pub async fn get_audit_log(
    State(state): State<AuditLogState>,
    Path(id): Path<Uuid>,
) -> Result<Json<Value>, AppError> {
    let audit_log = state
        .audit_log_service
        .get_by_id(id)
        .await?
        .ok_or_else(|| AppError::NotFound("Audit log not found".to_string()))?;

    Ok(Json(json!(audit_log)))
}

// POST /api/v1/admin/audit-logs
pub async fn create_audit_log(
    State(state): State<AuditLogState>,
    Json(payload): Json<CreateAuditLogRequest>,
) -> Result<(StatusCode, Json<Value>), AppError> {
    info!("create_audit_log: Creating new audit log entry");

    let audit_log = state.audit_log_service.create(payload).await?;

    info!(
        "create_audit_log: Successfully created audit log with id: {}",
        audit_log.id
    );
    Ok((
        StatusCode::CREATED,
        Json(json!({
            "message": "Audit log created successfully",
            "audit_log": audit_log
        })),
    ))
}

// GET /api/v1/admin/audit-logs/user/:user_id
pub async fn get_audit_logs_by_user(
    State(state): State<AuditLogState>,
    Path(user_id): Path<Uuid>,
    Query(query): Query<Value>,
) -> Result<Json<Value>, AppError> {
    let limit = query
        .get("limit")
        .and_then(|v| v.as_u64())
        .map(|v| v as i64);

    let logs = state
        .audit_log_service
        .get_by_user_id(user_id, limit)
        .await?;

    Ok(Json(json!({
        "logs": logs,
        "user_id": user_id,
        "total": logs.len()
    })))
}

// GET /api/v1/admin/audit-logs/resource/:resource_type/:resource_id
pub async fn get_audit_logs_by_resource(
    State(state): State<AuditLogState>,
    Path((resource_type, resource_id)): Path<(String, Uuid)>,
) -> Result<Json<Value>, AppError> {
    let logs = state
        .audit_log_service
        .get_by_resource(resource_type.clone(), resource_id)
        .await?;

    Ok(Json(json!({
        "logs": logs,
        "resource_type": resource_type,
        "resource_id": resource_id,
        "total": logs.len()
    })))
}

// GET /api/v1/admin/audit-logs/recent
pub async fn get_recent_audit_logs(
    State(state): State<AuditLogState>,
    Query(query): Query<Value>,
) -> Result<Json<Value>, AppError> {
    let limit = query
        .get("limit")
        .and_then(|v| v.as_u64())
        .map(|v| v as i64);

    let logs = state.audit_log_service.get_recent_logs(limit).await?;

    Ok(Json(json!({
        "logs": logs,
        "total": logs.len()
    })))
}

// GET /api/v1/admin/audit-logs/failed
pub async fn get_failed_audit_logs(
    State(state): State<AuditLogState>,
    Query(query): Query<Value>,
) -> Result<Json<Value>, AppError> {
    let limit = query
        .get("limit")
        .and_then(|v| v.as_u64())
        .map(|v| v as i64);

    let logs = state.audit_log_service.get_failed_actions(limit).await?;

    Ok(Json(json!({
        "logs": logs,
        "total": logs.len()
    })))
}

// DELETE /api/v1/admin/audit-logs/cleanup
pub async fn cleanup_old_audit_logs(
    State(state): State<AuditLogState>,
    Query(query): Query<Value>,
) -> Result<Json<Value>, AppError> {
    let days = query
        .get("days")
        .and_then(|v| v.as_u64())
        .map(|v| v as i32)
        .unwrap_or(365); // Default to 1 year

    if days < 30 {
        return Err(AppError::BadRequest(
            "Cannot delete logs newer than 30 days".to_string(),
        ));
    }

    let deleted_count = state.audit_log_service.delete_old_logs(days).await?;

    info!(
        "cleanup_old_audit_logs: Deleted {} logs older than {} days",
        deleted_count, days
    );
    Ok(Json(json!({
        "message": format!("Deleted {} old audit logs", deleted_count),
        "deleted_count": deleted_count,
        "days": days
    })))
}

// GET /api/v1/admin/audit-logs/stats
pub async fn get_audit_log_stats(
    State(state): State<AuditLogState>,
) -> Result<Json<Value>, AppError> {
    let stats = state.audit_log_service.get_stats().await?;

    Ok(Json(json!(stats)))
}
