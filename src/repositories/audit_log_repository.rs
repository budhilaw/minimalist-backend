use crate::models::audit_log::{
    AuditLog, AuditLogFilters, AuditLogResponse, CreateAuditLogRequest,
};
use anyhow::Result;
use sqlx::{PgPool, Row};
use uuid::Uuid;

pub struct AuditLogRepository {
    pool: PgPool,
}

impl AuditLogRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create(&self, request: CreateAuditLogRequest) -> Result<AuditLog> {
        let audit_log = sqlx::query_as!(
            AuditLog,
            r#"
            INSERT INTO audit_logs 
            (user_id, user_name, action, resource_type, resource_id, resource_title, 
             details, old_values, new_values, ip_address, user_agent, success, error_message)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            RETURNING id, user_id, user_name, action, resource_type, resource_id, 
                      resource_title, details, old_values, new_values, 
                      ip_address, user_agent, success, error_message, created_at
            "#,
            request.user_id,
            request.user_name,
            request.action,
            request.resource_type,
            request.resource_id,
            request.resource_title,
            request.details,
            request.old_values,
            request.new_values,
            request.ip_address,
            request.user_agent,
            request.success,
            request.error_message
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(audit_log)
    }

    pub async fn get_by_id(&self, id: Uuid) -> Result<Option<AuditLog>> {
        let audit_log = sqlx::query_as!(
            AuditLog,
            r#"
            SELECT id, user_id, user_name, action, resource_type, resource_id, 
                   resource_title, details, old_values, new_values, 
                   ip_address, user_agent, success, error_message, created_at
            FROM audit_logs 
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(audit_log)
    }

    pub async fn get_all_with_filters(&self, filters: AuditLogFilters) -> Result<AuditLogResponse> {
        let limit = filters.limit.unwrap_or(20);
        let offset = filters.offset.unwrap_or(0);
        let page = (offset / limit) + 1;

        // Build the WHERE clause dynamically
        let mut where_conditions = Vec::new();
        let mut params: Vec<Box<dyn sqlx::Encode<'_, sqlx::Postgres> + Send + Sync>> = Vec::new();
        let mut param_count = 0;

        if let Some(start_date) = filters.start_date {
            param_count += 1;
            where_conditions.push(format!("created_at >= ${}", param_count));
            params.push(Box::new(start_date));
        }

        if let Some(end_date) = filters.end_date {
            param_count += 1;
            where_conditions.push(format!("created_at <= ${}", param_count));
            params.push(Box::new(end_date));
        }

        if let Some(action) = filters.action {
            param_count += 1;
            where_conditions.push(format!("action = ${}", param_count));
            params.push(Box::new(action));
        }

        if let Some(resource_type) = filters.resource_type {
            param_count += 1;
            where_conditions.push(format!("resource_type = ${}", param_count));
            params.push(Box::new(resource_type));
        }

        if let Some(user_id) = filters.user_id {
            param_count += 1;
            where_conditions.push(format!("user_id = ${}", param_count));
            params.push(Box::new(user_id));
        }

        if let Some(success) = filters.success {
            param_count += 1;
            where_conditions.push(format!("success = ${}", param_count));
            params.push(Box::new(success));
        }

        if let Some(search) = filters.search {
            param_count += 1;
            where_conditions.push(format!(
                "(user_name ILIKE ${} OR details ILIKE ${} OR resource_title ILIKE ${})",
                param_count, param_count, param_count
            ));
            params.push(Box::new(format!("%{}%", search)));
        }

        let where_clause = if where_conditions.is_empty() {
            String::new()
        } else {
            format!("WHERE {}", where_conditions.join(" AND "))
        };

        // Get total count
        let count_query = format!("SELECT COUNT(*) as count FROM audit_logs {}", where_clause);

        let total_count: i64 = sqlx::query(&count_query)
            .fetch_one(&self.pool)
            .await?
            .get("count");

        // Get paginated results
        param_count += 1;
        let limit_param = param_count;
        param_count += 1;
        let offset_param = param_count;

        let data_query = format!(
            r#"
            SELECT id, user_id, user_name, action, resource_type, resource_id, 
                   resource_title, details, old_values, new_values, 
                   ip_address, user_agent, success, error_message, created_at
            FROM audit_logs 
            {} 
            ORDER BY created_at DESC 
            LIMIT ${} OFFSET ${}
            "#,
            where_clause, limit_param, offset_param
        );

        let logs = sqlx::query_as::<_, AuditLog>(&data_query)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;

        let total_pages = (total_count + limit - 1) / limit;

        Ok(AuditLogResponse {
            logs,
            total_count,
            page,
            per_page: limit,
            total_pages,
        })
    }

    pub async fn get_by_user_id(&self, user_id: Uuid, limit: Option<i64>) -> Result<Vec<AuditLog>> {
        let limit = limit.unwrap_or(50);

        let logs = sqlx::query_as!(
            AuditLog,
            r#"
            SELECT id, user_id, user_name, action, resource_type, resource_id, 
                   resource_title, details, old_values, new_values, 
                   ip_address, user_agent, success, error_message, created_at
            FROM audit_logs 
            WHERE user_id = $1 
            ORDER BY created_at DESC 
            LIMIT $2
            "#,
            user_id,
            limit
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(logs)
    }

    pub async fn get_by_resource(
        &self,
        resource_type: String,
        resource_id: Uuid,
    ) -> Result<Vec<AuditLog>> {
        let logs = sqlx::query_as!(
            AuditLog,
            r#"
            SELECT id, user_id, user_name, action, resource_type, resource_id, 
                   resource_title, details, old_values, new_values, 
                   ip_address, user_agent, success, error_message, created_at
            FROM audit_logs 
            WHERE resource_type = $1 AND resource_id = $2 
            ORDER BY created_at DESC
            "#,
            resource_type,
            resource_id
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(logs)
    }

    pub async fn get_recent_logs(&self, limit: Option<i64>) -> Result<Vec<AuditLog>> {
        let limit = limit.unwrap_or(10);

        let logs = sqlx::query_as!(
            AuditLog,
            r#"
            SELECT id, user_id, user_name, action, resource_type, resource_id, 
                   resource_title, details, old_values, new_values, 
                   ip_address, user_agent, success, error_message, created_at
            FROM audit_logs 
            ORDER BY created_at DESC 
            LIMIT $1
            "#,
            limit
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(logs)
    }

    pub async fn get_failed_actions(&self, limit: Option<i64>) -> Result<Vec<AuditLog>> {
        let limit = limit.unwrap_or(20);

        let logs = sqlx::query_as!(
            AuditLog,
            r#"
            SELECT id, user_id, user_name, action, resource_type, resource_id, 
                   resource_title, details, old_values, new_values, 
                   ip_address, user_agent, success, error_message, created_at
            FROM audit_logs 
            WHERE success = false 
            ORDER BY created_at DESC 
            LIMIT $1
            "#,
            limit
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(logs)
    }

    pub async fn delete_old_logs(&self, days: i32) -> Result<u64> {
        let result = sqlx::query!(
            "DELETE FROM audit_logs WHERE created_at < NOW() - INTERVAL '1 day' * $1",
            days as f64
        )
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }
}
