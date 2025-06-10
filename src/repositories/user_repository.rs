use anyhow::{Context, Result};
use async_trait::async_trait;
use sqlx::PgPool;
use uuid::Uuid;

use crate::models::user::{CreateUserRequest, UpdateProfileRequest, User};
use crate::utils::errors::AppError;

#[async_trait]
pub trait UserRepositoryTrait: Send + Sync {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, AppError>;
    async fn find_by_username(&self, username: &str) -> Result<Option<User>, AppError>;
    async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError>;
    async fn create(
        &self,
        user: CreateUserRequest,
        password_hash: String,
    ) -> Result<User, AppError>;
    async fn update_profile(
        &self,
        id: Uuid,
        update: UpdateProfileRequest,
    ) -> Result<User, AppError>;
    async fn update_password(&self, id: Uuid, password_hash: String) -> Result<(), AppError>;
    async fn update_last_login(&self, id: Uuid) -> Result<(), AppError>;
    async fn check_username_exists(
        &self,
        username: &str,
        exclude_id: Option<Uuid>,
    ) -> Result<bool, AppError>;
    async fn check_email_exists(
        &self,
        email: &str,
        exclude_id: Option<Uuid>,
    ) -> Result<bool, AppError>;
}

pub struct UserRepository {
    pool: PgPool,
}

impl UserRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserRepositoryTrait for UserRepository {
    async fn find_by_id(&self, id: Uuid) -> Result<Option<User>, AppError> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT id, username, email, password_hash, full_name, phone, role, 
                   is_active, last_login, created_at, updated_at
            FROM users 
            WHERE id = $1 AND is_active = true
            "#,
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch user by id")?;

        Ok(user)
    }

    async fn find_by_username(&self, username: &str) -> Result<Option<User>, AppError> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT id, username, email, password_hash, full_name, phone, role, 
                   is_active, last_login, created_at, updated_at
            FROM users 
            WHERE username = $1 AND is_active = true
            "#,
        )
        .bind(username)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch user by username")?;

        Ok(user)
    }

    async fn find_by_email(&self, email: &str) -> Result<Option<User>, AppError> {
        let user = sqlx::query_as::<_, User>(
            r#"
            SELECT id, username, email, password_hash, full_name, phone, role, 
                   is_active, last_login, created_at, updated_at
            FROM users 
            WHERE email = $1 AND is_active = true
            "#,
        )
        .bind(email)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to fetch user by email")?;

        Ok(user)
    }

    async fn create(
        &self,
        user: CreateUserRequest,
        password_hash: String,
    ) -> Result<User, AppError> {
        let created_user = sqlx::query_as::<_, User>(
            r#"
            INSERT INTO users (username, email, password_hash, full_name, phone, role)
            VALUES ($1, $2, $3, $4, $5, 'admin')
            RETURNING id, username, email, password_hash, full_name, phone, role, 
                      is_active, last_login, created_at, updated_at
            "#,
        )
        .bind(&user.username)
        .bind(&user.email)
        .bind(&password_hash)
        .bind(&user.full_name)
        .bind(&user.phone)
        .fetch_one(&self.pool)
        .await
        .context("Failed to create user")?;

        Ok(created_user)
    }

    async fn update_profile(
        &self,
        id: Uuid,
        update: UpdateProfileRequest,
    ) -> Result<User, AppError> {
        let updated_user = sqlx::query_as::<_, User>(
            r#"
            UPDATE users 
            SET full_name = $1, username = $2, email = $3, phone = $4, updated_at = NOW()
            WHERE id = $5 AND is_active = true
            RETURNING id, username, email, password_hash, full_name, phone, role, 
                      is_active, last_login, created_at, updated_at
            "#,
        )
        .bind(&update.full_name)
        .bind(&update.username)
        .bind(&update.email)
        .bind(&update.phone)
        .bind(id)
        .fetch_optional(&self.pool)
        .await
        .context("Failed to update user profile")?
        .ok_or(AppError::NotFound("User not found".to_string()))?;

        Ok(updated_user)
    }

    async fn update_password(&self, id: Uuid, password_hash: String) -> Result<(), AppError> {
        let result = sqlx::query(
            "UPDATE users SET password_hash = $1, updated_at = NOW() WHERE id = $2 AND is_active = true"
        )
        .bind(&password_hash)
        .bind(id)
        .execute(&self.pool)
        .await
        .context("Failed to update password")?;

        if result.rows_affected() == 0 {
            return Err(AppError::NotFound("User not found".to_string()));
        }

        Ok(())
    }

    async fn update_last_login(&self, id: Uuid) -> Result<(), AppError> {
        sqlx::query("UPDATE users SET last_login = NOW() WHERE id = $1")
            .bind(id)
            .execute(&self.pool)
            .await
            .context("Failed to update last login")?;

        Ok(())
    }

    async fn check_username_exists(
        &self,
        username: &str,
        exclude_id: Option<Uuid>,
    ) -> Result<bool, AppError> {
        let query = match exclude_id {
            Some(id) => sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM users WHERE username = $1 AND id != $2 AND is_active = true",
            )
            .bind(username)
            .bind(id),
            None => sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM users WHERE username = $1 AND is_active = true",
            )
            .bind(username),
        };

        let count = query
            .fetch_one(&self.pool)
            .await
            .context("Failed to check username existence")?;

        Ok(count > 0)
    }

    async fn check_email_exists(
        &self,
        email: &str,
        exclude_id: Option<Uuid>,
    ) -> Result<bool, AppError> {
        let query = match exclude_id {
            Some(id) => sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM users WHERE email = $1 AND id != $2 AND is_active = true",
            )
            .bind(email)
            .bind(id),
            None => sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(*) FROM users WHERE email = $1 AND is_active = true",
            )
            .bind(email),
        };

        let count = query
            .fetch_one(&self.pool)
            .await
            .context("Failed to check email existence")?;

        Ok(count > 0)
    }
}
