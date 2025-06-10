use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;
use anyhow::Result;
use serde_json::json;

use crate::{
    models::audit_log::{AuditLog, CreateAuditLogRequest, AuditLogFilters, AuditLogResponse},
    repositories::AuditLogRepository,
};

#[async_trait]
pub trait AuditLogServiceTrait: Send + Sync {
    async fn create(&self, request: CreateAuditLogRequest) -> Result<AuditLog>;
    async fn get_by_id(&self, id: Uuid) -> Result<Option<AuditLog>>;
    async fn get_all_with_filters(&self, filters: AuditLogFilters) -> Result<AuditLogResponse>;
    async fn get_by_user_id(&self, user_id: Uuid, limit: Option<i64>) -> Result<Vec<AuditLog>>;
    async fn get_by_resource(&self, resource_type: String, resource_id: Uuid) -> Result<Vec<AuditLog>>;
    async fn get_recent_logs(&self, limit: Option<i64>) -> Result<Vec<AuditLog>>;
    async fn get_failed_actions(&self, limit: Option<i64>) -> Result<Vec<AuditLog>>;
    async fn delete_old_logs(&self, days: i32) -> Result<u64>;
    async fn get_stats(&self) -> Result<serde_json::Value>;
    
    // Helper methods
    async fn log_admin_action(
        &self,
        user_id: Option<Uuid>,
        user_name: Option<String>,
        action: &str,
        resource_type: &str,
        resource_id: Option<Uuid>,
        resource_title: Option<String>,
        details: Option<String>,
        old_values: Option<serde_json::Value>,
        new_values: Option<serde_json::Value>,
        success: bool,
        error_message: Option<String>,
    ) -> Result<AuditLog>;
    
    async fn log_auth_event(
        &self,
        user_id: Option<Uuid>,
        user_name: Option<String>,
        action: &str,
        success: bool,
        details: Option<String>,
        error_message: Option<String>,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Result<AuditLog>;
}

pub struct AuditLogService {
    repository: Arc<AuditLogRepository>,
}

impl AuditLogService {
    pub fn new(repository: Arc<AuditLogRepository>) -> Self {
        Self { repository }
    }

    // Helper method to create audit log for admin actions
    pub async fn log_admin_action(
        &self,
        user_id: Option<Uuid>,
        user_name: Option<String>,
        action: &str,
        resource_type: &str,
        resource_id: Option<Uuid>,
        resource_title: Option<String>,
        details: Option<String>,
        old_values: Option<serde_json::Value>,
        new_values: Option<serde_json::Value>,
        success: bool,
        error_message: Option<String>,
    ) -> Result<AuditLog> {
        let request = CreateAuditLogRequest {
            user_id,
            user_name,
            action: action.to_string(),
            resource_type: resource_type.to_string(),
            resource_id,
            resource_title,
            details,
            old_values,
            new_values,
            ip_address: None, // This should be extracted from request context
            user_agent: None, // This should be extracted from request context
            success,
            error_message,
        };

        self.create(request).await
    }

    // Helper method to log authentication events
    pub async fn log_auth_event(
        &self,
        user_id: Option<Uuid>,
        user_name: Option<String>,
        action: &str,
        success: bool,
        details: Option<String>,
        error_message: Option<String>,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Result<AuditLog> {
        let request = CreateAuditLogRequest {
            user_id,
            user_name,
            action: action.to_string(),
            resource_type: "authentication".to_string(),
            resource_id: None,
            resource_title: None,
            details,
            old_values: None,
            new_values: None,
            ip_address: ip_address.and_then(|ip| ip.parse().ok()),
            user_agent,
            success,
            error_message,
        };

        self.create(request).await
    }

    // Helper method to log CRUD operations
    pub async fn log_crud_operation(
        &self,
        user_id: Option<Uuid>,
        user_name: Option<String>,
        action: &str,
        resource_type: &str,
        resource_id: Uuid,
        resource_title: Option<String>,
        old_values: Option<serde_json::Value>,
        new_values: Option<serde_json::Value>,
    ) -> Result<AuditLog> {
        self.log_admin_action(
            user_id,
            user_name,
            action,
            resource_type,
            Some(resource_id),
            resource_title,
            Some(format!("{} operation on {}", action, resource_type)),
            old_values,
            new_values,
            true,
            None,
        ).await
    }
}

#[async_trait]
impl AuditLogServiceTrait for AuditLogService {
    async fn create(&self, request: CreateAuditLogRequest) -> Result<AuditLog> {
        self.repository.create(request).await
    }

    async fn get_by_id(&self, id: Uuid) -> Result<Option<AuditLog>> {
        self.repository.get_by_id(id).await
    }

    async fn get_all_with_filters(&self, filters: AuditLogFilters) -> Result<AuditLogResponse> {
        self.repository.get_all_with_filters(filters).await
    }

    async fn get_by_user_id(&self, user_id: Uuid, limit: Option<i64>) -> Result<Vec<AuditLog>> {
        self.repository.get_by_user_id(user_id, limit).await
    }

    async fn get_by_resource(&self, resource_type: String, resource_id: Uuid) -> Result<Vec<AuditLog>> {
        self.repository.get_by_resource(resource_type, resource_id).await
    }

    async fn get_recent_logs(&self, limit: Option<i64>) -> Result<Vec<AuditLog>> {
        self.repository.get_recent_logs(limit).await
    }

    async fn get_failed_actions(&self, limit: Option<i64>) -> Result<Vec<AuditLog>> {
        self.repository.get_failed_actions(limit).await
    }

    async fn delete_old_logs(&self, days: i32) -> Result<u64> {
        if days < 30 {
            return Err(anyhow::anyhow!("Cannot delete logs newer than 30 days"));
        }
        self.repository.delete_old_logs(days).await
    }

    async fn get_stats(&self) -> Result<serde_json::Value> {
        // Get various statistics about audit logs
        let recent_logs = self.repository.get_recent_logs(Some(100)).await?;
        let failed_logs = self.repository.get_failed_actions(Some(50)).await?;
        
        // Calculate statistics
        let total_recent = recent_logs.len();
        let total_failed = failed_logs.len();
        let success_rate = if total_recent > 0 {
            ((total_recent - total_failed) as f64 / total_recent as f64) * 100.0
        } else {
            100.0
        };

        // Count actions by type
        let mut action_counts = std::collections::HashMap::new();
        let mut resource_counts = std::collections::HashMap::new();
        let mut user_counts = std::collections::HashMap::new();

        for log in &recent_logs {
            *action_counts.entry(log.action.clone()).or_insert(0) += 1;
            *resource_counts.entry(log.resource_type.clone()).or_insert(0) += 1;
            if let Some(user_name) = &log.user_name {
                *user_counts.entry(user_name.clone()).or_insert(0) += 1;
            }
        }

        // Get top actions, resources, and users
        let mut top_actions: Vec<_> = action_counts.into_iter().collect();
        top_actions.sort_by(|a, b| b.1.cmp(&a.1));
        top_actions.truncate(5);

        let mut top_resources: Vec<_> = resource_counts.into_iter().collect();
        top_resources.sort_by(|a, b| b.1.cmp(&a.1));
        top_resources.truncate(5);

        let mut top_users: Vec<_> = user_counts.into_iter().collect();
        top_users.sort_by(|a, b| b.1.cmp(&a.1));
        top_users.truncate(5);

        Ok(json!({
            "summary": {
                "total_recent_logs": total_recent,
                "total_failed_logs": total_failed,
                "success_rate": format!("{:.1}%", success_rate)
            },
            "top_actions": top_actions.into_iter().map(|(action, count)| json!({
                "action": action,
                "count": count
            })).collect::<Vec<_>>(),
            "top_resources": top_resources.into_iter().map(|(resource, count)| json!({
                "resource_type": resource,
                "count": count
            })).collect::<Vec<_>>(),
            "top_users": top_users.into_iter().map(|(user, count)| json!({
                "user_name": user,
                "count": count
            })).collect::<Vec<_>>(),
            "recent_failed_actions": failed_logs.into_iter().take(10).map(|log| json!({
                "id": log.id,
                "action": log.action,
                "resource_type": log.resource_type,
                "user_name": log.user_name,
                "error_message": log.error_message,
                "created_at": log.created_at
            })).collect::<Vec<_>>()
        }))
    }

    async fn log_admin_action(
        &self,
        user_id: Option<Uuid>,
        user_name: Option<String>,
        action: &str,
        resource_type: &str,
        resource_id: Option<Uuid>,
        resource_title: Option<String>,
        details: Option<String>,
        old_values: Option<serde_json::Value>,
        new_values: Option<serde_json::Value>,
        success: bool,
        error_message: Option<String>,
    ) -> Result<AuditLog> {
        let request = CreateAuditLogRequest {
            user_id,
            user_name,
            action: action.to_string(),
            resource_type: resource_type.to_string(),
            resource_id,
            resource_title,
            details,
            old_values,
            new_values,
            ip_address: None,
            user_agent: None,
            success,
            error_message,
        };

        self.create(request).await
    }

    async fn log_auth_event(
        &self,
        user_id: Option<Uuid>,
        user_name: Option<String>,
        action: &str,
        success: bool,
        details: Option<String>,
        error_message: Option<String>,
        ip_address: Option<String>,
        user_agent: Option<String>,
    ) -> Result<AuditLog> {
        let request = CreateAuditLogRequest {
            user_id,
            user_name,
            action: action.to_string(),
            resource_type: "authentication".to_string(),
            resource_id: None,
            resource_title: None,
            details,
            old_values: None,
            new_values: None,
            ip_address: ip_address.and_then(|ip| ip.parse().ok()),
            user_agent,
            success,
            error_message,
        };

        self.create(request).await
    }
} 