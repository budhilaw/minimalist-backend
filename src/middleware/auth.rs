use axum::{
    extract::{Request, State},
    http::header::AUTHORIZATION,
    middleware::Next,
    response::Response,
};
use axum_extra::extract::TypedHeader;
use headers::{authorization::Bearer, Authorization};

use crate::services::auth_service::{AuthService, Claims};
use crate::utils::errors::AppError;

pub async fn auth_middleware(
    State(auth_service): State<AuthService>,
    TypedHeader(authorization): TypedHeader<Authorization<Bearer>>,
    mut request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let token = authorization.token();

    let claims = auth_service
        .validate_token(token)
        .map_err(|_| AppError::Unauthorized("Invalid or expired token".to_string()))?;

    // Add claims to request extensions so handlers can access them
    request.extensions_mut().insert(claims);

    Ok(next.run(request).await)
}

pub async fn optional_auth_middleware(
    State(auth_service): State<AuthService>,
    request: Request,
    next: Next,
) -> Response {
    let mut request = request;

    // Try to extract authorization header
    if let Some(auth_header) = request.headers().get(AUTHORIZATION) {
        if let Ok(auth_str) = auth_header.to_str() {
            if let Some(token) = auth_str.strip_prefix("Bearer ") {
                if let Ok(claims) = auth_service.validate_token(token) {
                    request.extensions_mut().insert(claims);
                }
            }
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
