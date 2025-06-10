use anyhow::Result;
use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;
use validator::Validate;

use crate::models::user::{LoginRequest, LoginResponse, User, UserResponse};
use crate::repositories::user_repository::UserRepositoryTrait;
use crate::utils::{errors::AppError, password::PasswordService};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String, // user_id
    pub username: String,
    pub role: String,
    pub exp: i64,
    pub iat: i64,
}

#[derive(Clone)]
pub struct AuthService {
    user_repository: Arc<dyn UserRepositoryTrait>,
    jwt_secret: String,
    token_expiry: i64,
    password_service: PasswordService,
}

impl AuthService {
    pub fn new(
        user_repository: Arc<dyn UserRepositoryTrait>,
        jwt_secret: String,
        token_expiry: i64,
    ) -> Self {
        Self {
            user_repository,
            jwt_secret,
            token_expiry,
            password_service: PasswordService::new(),
        }
    }

    pub async fn authenticate_user(
        &self,
        request: LoginRequest,
    ) -> Result<LoginResponse, AppError> {
        // Validate request
        request.validate()?;

        // Rate limiting should be handled at middleware level
        let user = self
            .user_repository
            .find_by_username(&request.username)
            .await?
            .ok_or(AppError::Unauthorized("Invalid credentials".to_string()))?;

        // Verify password
        let is_valid = self
            .password_service
            .verify_password(&request.password, &user.password_hash)?;

        if !is_valid {
            return Err(AppError::Unauthorized("Invalid credentials".to_string()));
        }

        // Update last login
        self.user_repository.update_last_login(user.id).await?;

        // Generate JWT token
        let (token, expires_at) = self.generate_token(&user)?;

        Ok(LoginResponse {
            token,
            user: user.into(),
            expires_at,
        })
    }

    pub fn generate_token(&self, user: &User) -> Result<(String, chrono::DateTime<Utc>), AppError> {
        let now = Utc::now();
        let expiration = now + Duration::seconds(self.token_expiry);

        let claims = Claims {
            sub: user.id.to_string(),
            username: user.username.clone(),
            role: user.role.clone(),
            exp: expiration.timestamp(),
            iat: now.timestamp(),
        };

        let token = encode(
            &Header::default(),
            &claims,
            &EncodingKey::from_secret(self.jwt_secret.as_ref()),
        )
        .map_err(|_| AppError::Internal("Failed to generate token".to_string()))?;

        Ok((token, expiration))
    }

    pub fn validate_token(&self, token: &str) -> Result<Claims, AppError> {
        let token_data = decode::<Claims>(
            token,
            &DecodingKey::from_secret(self.jwt_secret.as_ref()),
            &Validation::default(),
        )
        .map_err(|_| AppError::Unauthorized("Invalid or expired token".to_string()))?;

        Ok(token_data.claims)
    }

    pub async fn get_user_by_id(&self, user_id: Uuid) -> Result<User, AppError> {
        let user = self
            .user_repository
            .find_by_id(user_id)
            .await?
            .ok_or(AppError::NotFound("User not found".to_string()))?;

        Ok(user)
    }

    pub async fn refresh_token(&self, old_token: &str) -> Result<LoginResponse, AppError> {
        let claims = self.validate_token(old_token)?;
        let user_id = Uuid::parse_str(&claims.sub)
            .map_err(|_| AppError::Internal("Invalid user ID in token".to_string()))?;

        let user = self.get_user_by_id(user_id).await?;
        let (token, expires_at) = self.generate_token(&user)?;

        Ok(LoginResponse {
            token,
            user: user.into(),
            expires_at,
        })
    }

    pub async fn update_profile(
        &self,
        user_id: Uuid,
        request: crate::models::user::UpdateProfileRequest,
    ) -> Result<UserResponse, AppError> {
        // Validate request
        request.validate()?;

        // Check if username or email already exists for another user
        let username_exists = self
            .user_repository
            .check_username_exists(&request.username, Some(user_id))
            .await?;

        if username_exists {
            return Err(AppError::Conflict("Username already exists".to_string()));
        }

        let email_exists = self
            .user_repository
            .check_email_exists(&request.email, Some(user_id))
            .await?;

        if email_exists {
            return Err(AppError::Conflict("Email already exists".to_string()));
        }

        // Update user profile
        let updated_user = self
            .user_repository
            .update_profile(user_id, request)
            .await?;

        Ok(updated_user.into())
    }

    pub async fn change_password(
        &self,
        user_id: Uuid,
        request: crate::models::user::ChangePasswordRequest,
    ) -> Result<(), AppError> {
        // Validate request
        request.validate()?;

        // Verify password strength
        if !PasswordService::is_strong_password(&request.new_password) {
            return Err(AppError::Validation(
                "Password must be at least 8 characters and contain a mix of uppercase, lowercase, numbers, and special characters".to_string()
            ));
        }

        // Get current user
        let user = self.get_user_by_id(user_id).await?;

        // Verify current password
        let is_valid = self
            .password_service
            .verify_password(&request.current_password, &user.password_hash)?;

        if !is_valid {
            return Err(AppError::Unauthorized(
                "Current password is incorrect".to_string(),
            ));
        }

        // Hash new password
        let new_hash = self.password_service.hash_password(&request.new_password)?;

        // Update password using repository
        self.user_repository
            .update_password(user_id, new_hash)
            .await?;

        Ok(())
    }
}
