use axum::{
    extract::{Request, State},
    http::header::{AUTHORIZATION, COOKIE},
    middleware::Next,
    response::Response,
};

use crate::services::auth_service::{AuthService, Claims};
use crate::utils::errors::AppError;

pub async fn auth_middleware(
    State(auth_service): State<AuthService>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    // Try to get token from cookie first, then fallback to Authorization header
    let token = if let Some(cookie_header) = request.headers().get(COOKIE) {
        if let Ok(cookie_str) = cookie_header.to_str() {
            extract_admin_token_from_cookies(cookie_str)
        } else {
            None
        }
    } else {
        None
    };

    // Fallback to Authorization header if no cookie token found
    let token = token.or_else(|| {
        request
            .headers()
            .get(AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "))
            .map(|s| s.to_string())
    });

    let token =
        token.ok_or_else(|| AppError::Unauthorized("Missing authentication token".to_string()))?;

    let claims = auth_service
        .validate_token(&token)
        .map_err(|_| AppError::Unauthorized("Invalid or expired token".to_string()))?;

    // Add claims to request extensions so handlers can access them
    request.extensions_mut().insert(claims);

    Ok(next.run(request).await)
}

// Helper function to extract admin_token from cookie string
fn extract_admin_token_from_cookies(cookie_str: &str) -> Option<String> {
    for cookie in cookie_str.split(';') {
        let cookie = cookie.trim();
        if let Some(value) = cookie.strip_prefix("admin_token=") {
            return Some(value.to_string());
        }
    }
    None
}

pub async fn optional_auth_middleware(
    State(auth_service): State<AuthService>,
    request: Request,
    next: Next,
) -> Response {
    let mut request = request;

    // Try to extract token from cookie first
    let token = if let Some(cookie_header) = request.headers().get(COOKIE) {
        if let Ok(cookie_str) = cookie_header.to_str() {
            extract_admin_token_from_cookies(cookie_str)
        } else {
            None
        }
    } else {
        None
    };

    // Fallback to Authorization header
    let token = token.or_else(|| {
        request
            .headers()
            .get(AUTHORIZATION)
            .and_then(|h| h.to_str().ok())
            .and_then(|s| s.strip_prefix("Bearer "))
            .map(|s| s.to_string())
    });

    if let Some(token) = token {
        if let Ok(claims) = auth_service.validate_token(&token) {
            request.extensions_mut().insert(claims);
        }
    }

    next.run(request).await
}

// Axum extractor for getting claims from request
use axum::{async_trait, extract::FromRequestParts, http::request::Parts};

#[async_trait]
impl<S> FromRequestParts<S> for Claims
where
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<Claims>()
            .cloned()
            .ok_or(AppError::Unauthorized("Missing authentication".to_string()))
    }
}
