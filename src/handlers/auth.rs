use axum::{
    extract::{ConnectInfo, State},
    http::{header::SET_COOKIE, HeaderMap},
    response::Json,
};
use serde_json::json;
use std::{net::SocketAddr, sync::Arc};
use uuid::Uuid;

use crate::middleware::rate_limiter::{
    check_and_auto_block_ip, clear_auth_rate_limit, record_auth_failure, RedisRateLimiter,
};
use crate::models::user::{ChangePasswordRequest, LoginRequest, UpdateProfileRequest};
use crate::services::audit_log_service::AuditLogServiceTrait;
use crate::services::auth_service::{AuthService, Claims};
use crate::utils::errors::AppError;

// State struct to hold auth service, audit log service, and rate limiter
#[derive(Clone)]
pub struct AuthState {
    pub auth_service: AuthService,
    pub audit_log_service: Arc<dyn AuditLogServiceTrait>,
    pub rate_limiter: Option<Arc<RedisRateLimiter>>,
}

pub async fn login(
    State(state): State<AuthState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    Json(request): Json<LoginRequest>,
) -> Result<axum::response::Response, AppError> {
    let username = request.username.clone();
    let client_ip = get_client_ip(&headers, Some(&addr));
    let user_agent = get_user_agent(&headers);

    // Check if IP is manually blocked (simple Redis check)
    if let Some(ref limiter) = state.rate_limiter {
        let blocked_key = format!("blocked_ip:{}", client_ip);
        if let Ok(mut conn) = limiter.get_connection().await {
            let is_blocked: Option<String> = redis::cmd("GET")
                .arg(&blocked_key)
                .query_async(&mut conn)
                .await
                .unwrap_or(None);

            if is_blocked.is_some() {
                return Err(AppError::TooManyRequests {
                    message: "Your IP address has been blocked due to suspicious activity. Please contact support if you believe this is an error.".to_string(),
                    retry_after: None,
                });
            }
        }
    }

    // Check rate limiting before authentication
    if let Some(ref limiter) = state.rate_limiter {
        match limiter
            .check_auth_rate_limit(&client_ip, Some(&username))
            .await
        {
            Ok((allowed, info)) => {
                if !allowed {
                    return Err(AppError::TooManyRequests {
                        message: info
                            .reason
                            .unwrap_or_else(|| "Too many authentication attempts".to_string()),
                        retry_after: info.lockout_seconds,
                    });
                }
            }
            Err(e) => {
                tracing::warn!("Rate limiter check failed: {}", e);
                // Continue without rate limiting if Redis is down
            }
        }
    }

    match state.auth_service.authenticate_user(request).await {
        Ok(response) => {
            // Clear rate limiting on successful authentication
            if let Some(ref limiter) = state.rate_limiter {
                if let Err(e) = clear_auth_rate_limit(limiter, &client_ip, &username).await {
                    tracing::warn!("Failed to clear auth rate limit: {}", e);
                }
            }

            // Log successful login
            if let Err(e) = state
                .audit_log_service
                .log_auth_event(
                    Some(response.user.id),
                    Some(response.user.username.clone()),
                    "login",
                    true,
                    Some(format!(
                        "Successful login for user: {}",
                        response.user.username
                    )),
                    None,
                    Some(client_ip.clone()),
                    user_agent.clone(),
                )
                .await
            {
                eprintln!("Failed to log successful login: {}", e);
            }

            // Create secure httpOnly cookie for the token
            let cookie_value = format!(
                "admin_token={}; HttpOnly; Secure; SameSite=Strict; Path=/; Max-Age={}",
                response.token,
                24 * 60 * 60 // 24 hours in seconds
            );

            // Build response with cookie
            let json_response = Json(json!({
                "success": true,
                "data": {
                    "user": response.user,
                    "expires_at": response.expires_at,
                    // Don't send token in response body for security
                }
            }));

            let mut response = axum::response::Response::new(
                serde_json::to_string(&json_response.0).unwrap().into(),
            );

            response.headers_mut().insert(
                axum::http::header::CONTENT_TYPE,
                "application/json".parse().unwrap(),
            );

            response
                .headers_mut()
                .insert(SET_COOKIE, cookie_value.parse().unwrap());

            Ok(response)
        }
        Err(e) => {
            // Record failed attempt and check for auto-blocking
            if let Some(ref limiter) = state.rate_limiter {
                if let Err(redis_err) = record_auth_failure(limiter, &client_ip, &username).await {
                    tracing::warn!("Failed to record auth failure: {}", redis_err);
                } else {
                    // Check if we should auto-block this IP
                    if let Err(block_err) = check_and_auto_block_ip(limiter, &client_ip).await {
                        tracing::warn!("Failed to check auto-block: {}", block_err);
                    }
                }
            }

            // Log failed login attempt
            if let Err(log_err) = state
                .audit_log_service
                .log_auth_event(
                    None,
                    Some(username.clone()),
                    "login_failed",
                    false,
                    Some(format!("Failed login attempt for username: {}", username)),
                    Some(e.to_string()),
                    Some(client_ip.clone()),
                    user_agent.clone(),
                )
                .await
            {
                eprintln!("Failed to log failed login: {}", log_err);
            }

            Err(e)
        }
    }
}

pub async fn logout(
    State(state): State<AuthState>,
    claims: Claims,
) -> Result<axum::response::Response, AppError> {
    // Log logout
    if let Err(e) = state
        .audit_log_service
        .log_auth_event(
            Some(Uuid::parse_str(&claims.sub).unwrap_or_default()),
            Some(claims.username.clone()),
            "logout",
            true,
            Some(format!("User {} logged out", claims.username)),
            None,
            None, // IP not available for logout endpoint
            None, // User agent not available for logout endpoint
        )
        .await
    {
        eprintln!("Failed to log logout: {}", e);
    }

    // Clear the cookie by setting it to expire
    let clear_cookie = "admin_token=; HttpOnly; Secure; SameSite=Strict; Path=/; Max-Age=0";

    let json_response = Json(json!({
        "success": true,
        "message": "Successfully logged out"
    }));

    let mut response =
        axum::response::Response::new(serde_json::to_string(&json_response.0).unwrap().into());

    response.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        "application/json".parse().unwrap(),
    );

    response
        .headers_mut()
        .insert(SET_COOKIE, clear_cookie.parse().unwrap());

    Ok(response)
}

// Helper function to extract client IP
fn get_client_ip(headers: &HeaderMap, addr: Option<&SocketAddr>) -> String {
    // Priority: X-Forwarded-For > X-Real-IP > actual connection IP > fallback to unknown
    if let Some(forwarded) = headers.get("x-forwarded-for") {
        if let Ok(forwarded_str) = forwarded.to_str() {
            if let Some(first_ip) = forwarded_str.split(',').next() {
                return first_ip.trim().to_string();
            }
        }
    }

    if let Some(real_ip) = headers.get("x-real-ip") {
        if let Ok(ip_str) = real_ip.to_str() {
            return ip_str.to_string();
        }
    }

    // Use actual connection IP if available
    if let Some(socket_addr) = addr {
        return socket_addr.ip().to_string();
    }

    "unknown".to_string()
}

// Helper function to extract user agent
fn get_user_agent(headers: &HeaderMap) -> Option<String> {
    headers
        .get("user-agent")
        .and_then(|h| h.to_str().ok())
        .map(|s| s.to_string())
}

pub async fn me(
    State(state): State<AuthState>,
    claims: Claims,
) -> Result<Json<serde_json::Value>, AppError> {
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Internal("Invalid user ID in token".to_string()))?;

    let user = state.auth_service.get_user_by_id(user_id).await?;

    Ok(Json(json!({
        "success": true,
        "data": {
            "user": crate::models::user::UserResponse::from(user)
        }
    })))
}

pub async fn refresh_token(
    State(state): State<AuthState>,
    claims: Claims,
) -> Result<Json<serde_json::Value>, AppError> {
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Internal("Invalid user ID in token".to_string()))?;

    let user = state.auth_service.get_user_by_id(user_id).await?;
    let (token, expires_at) = state.auth_service.generate_token(&user)?;

    Ok(Json(json!({
        "success": true,
        "data": {
            "token": token,
            "expires_at": expires_at,
            "user": crate::models::user::UserResponse::from(user)
        }
    })))
}

pub async fn update_profile(
    State(state): State<AuthState>,
    claims: Claims,
    Json(request): Json<UpdateProfileRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Internal("Invalid user ID in token".to_string()))?;

    let updated_user = state.auth_service.update_profile(user_id, request).await?;

    // Log profile update
    if let Err(e) = state
        .audit_log_service
        .log_admin_action(
            Some(user_id),
            Some(claims.username.clone()),
            "profile_updated",
            "profile",
            Some(user_id),
            Some(format!("Profile for {}", claims.username)),
            Some("User profile updated".to_string()),
            None,
            None,
            true,
            None,
        )
        .await
    {
        eprintln!("Failed to log profile update: {}", e);
    }

    Ok(Json(json!({
        "success": true,
        "data": {
            "user": updated_user
        },
        "message": "Profile updated successfully"
    })))
}

pub async fn change_password(
    State(state): State<AuthState>,
    claims: Claims,
    Json(request): Json<ChangePasswordRequest>,
) -> Result<axum::response::Response, AppError> {
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AppError::Internal("Invalid user ID in token".to_string()))?;

    match state.auth_service.change_password(user_id, request).await {
        Ok(_) => {
            // Log successful password change
            if let Err(e) = state
                .audit_log_service
                .log_admin_action(
                    Some(user_id),
                    Some(claims.username.clone()),
                    "password_changed",
                    "authentication",
                    Some(user_id),
                    Some(format!("Password for {}", claims.username)),
                    Some("Password changed successfully - user logged out for security".to_string()),
                    None,
                    None,
                    true,
                    None,
                )
                .await
            {
                eprintln!("Failed to log password change: {}", e);
            }

            // Clear the authentication cookie for security
            let clear_cookie = "admin_token=; HttpOnly; Secure; SameSite=Strict; Path=/; Max-Age=0";

            let json_response = Json(json!({
                "success": true,
                "message": "Password changed successfully. Please log in again with your new password.",
                "requires_reauth": true
            }));

            let mut response =
                axum::response::Response::new(serde_json::to_string(&json_response.0).unwrap().into());

            response.headers_mut().insert(
                axum::http::header::CONTENT_TYPE,
                "application/json".parse().unwrap(),
            );

            // Clear the authentication cookie
            response
                .headers_mut()
                .insert(SET_COOKIE, clear_cookie.parse().unwrap());

            Ok(response)
        }
        Err(e) => {
            // Log failed password change
            if let Err(log_err) = state
                .audit_log_service
                .log_admin_action(
                    Some(user_id),
                    Some(claims.username.clone()),
                    "password_change_failed",
                    "authentication",
                    Some(user_id),
                    Some(format!("Password for {}", claims.username)),
                    Some("Password change failed".to_string()),
                    None,
                    None,
                    false,
                    Some(e.to_string()),
                )
                .await
            {
                eprintln!("Failed to log failed password change: {}", log_err);
            }

            Err(e)
        }
    }
}
