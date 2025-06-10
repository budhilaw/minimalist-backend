use axum::{
    extract::Request,
    http::{header, HeaderValue, Method},
    middleware::Next,
    response::Response,
};
use std::{sync::Arc, time::Duration};

use crate::utils::{config::SecurityConfig, errors::AppError};
use super::rate_limiter::RedisRateLimiter;

// Create rate limiter with Redis backend
pub async fn create_rate_limiter(
    security_config: &SecurityConfig, 
    redis_url: &str
) -> Result<Arc<RedisRateLimiter>, Box<dyn std::error::Error + Send + Sync>> {
    // Configure rate limiting parameters
    let limiter = RedisRateLimiter::new(
        redis_url,
        // Authentication rate limiting
        20,    // auth_ip_limit: 20 attempts per IP
        300,   // auth_ip_window_seconds: 5 minutes
        5,     // auth_user_limit: 5 attempts per username
        900,   // auth_user_window_seconds: 15 minutes
        
        // IP blocking thresholds
        5,     // ip_block_threshold: Block after 5 total failures
        24,    // ip_block_duration_hours: Block for 24 hours (0 = permanent)
        
        // General API rate limiting
        security_config.rate_limit.requests_per_minute as u32,
        60,    // api_window_seconds: 1 minute
    )?;
    
    Ok(Arc::new(limiter))
}

// Fallback no-op rate limiter for when Redis is not available
pub fn create_noop_rate_limiter() -> tower::layer::util::Identity {
    tracing::warn!("Redis not available, using no-op rate limiter");
    tower::layer::util::Identity::new()
}

pub async fn security_headers_middleware(
    request: Request,
    next: Next,
) -> Result<Response, AppError> {
    let mut response = next.run(request).await;

    let headers = response.headers_mut();

    // Security headers
    headers.insert(
        header::HeaderName::from_static("x-frame-options"),
        HeaderValue::from_static("DENY"),
    );

    headers.insert(
        header::HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );

    headers.insert(
        header::HeaderName::from_static("x-xss-protection"),
        HeaderValue::from_static("1; mode=block"),
    );

    headers.insert(
        header::HeaderName::from_static("strict-transport-security"),
        HeaderValue::from_static("max-age=31536000; includeSubDomains"),
    );

    headers.insert(
        header::HeaderName::from_static("referrer-policy"),
        HeaderValue::from_static("strict-origin-when-cross-origin"),
    );

    headers.insert(
        header::HeaderName::from_static("content-security-policy"),
        HeaderValue::from_static(
            "default-src 'self'; script-src 'self' 'unsafe-inline'; style-src 'self' 'unsafe-inline'; img-src 'self' data: https:; font-src 'self'; connect-src 'self'; frame-ancestors 'none';"
        ),
    );

    headers.insert(
        header::HeaderName::from_static("permissions-policy"),
        HeaderValue::from_static(
            "camera=(), microphone=(), location=(), payment=(), usb=(), magnetometer=(), gyroscope=(), accelerometer=()"
        ),
    );

    Ok(response)
}

pub async fn request_id_middleware(mut request: Request, next: Next) -> Result<Response, AppError> {
    let request_id = uuid::Uuid::new_v4().to_string();

    // Add request ID to request headers for logging
    request.headers_mut().insert(
        header::HeaderName::from_static("x-request-id"),
        HeaderValue::from_str(&request_id).unwrap(),
    );

    let mut response = next.run(request).await;

    // Add request ID to response headers
    response.headers_mut().insert(
        header::HeaderName::from_static("x-request-id"),
        HeaderValue::from_str(&request_id).unwrap(),
    );

    Ok(response)
}

pub async fn logging_middleware(request: Request, next: Next) -> Response {
    let start = std::time::Instant::now();
    let method = request.method().clone();
    let uri = request.uri().clone();

    // Extract request ID before moving the request
    let request_id = request
        .headers()
        .get("x-request-id")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("unknown")
        .to_string();

    tracing::info!(
        request_id = %request_id,
        method = %method,
        uri = %uri,
        "Request started"
    );

    let response = next.run(request).await;

    let status = response.status();
    let duration = start.elapsed();

    tracing::info!(
        request_id = %request_id,
        method = %method,
        uri = %uri,
        status = %status,
        duration_ms = %duration.as_millis(),
        "Request completed"
    );

    response
}

// Custom CORS middleware with configuration
pub fn create_cors_layer(security_config: &SecurityConfig) -> tower_http::cors::CorsLayer {
    use tower_http::cors::{Any, CorsLayer};

    let mut cors = CorsLayer::new();
    let allow_any_origin = security_config
        .cors
        .allowed_origins
        .contains(&"*".to_string());

    // Configure allowed origins
    if allow_any_origin {
        cors = cors.allow_origin(Any);
    } else {
        for origin in &security_config.cors.allowed_origins {
            if let Ok(header_value) = origin.parse::<HeaderValue>() {
                cors = cors.allow_origin(header_value);
            }
        }
    }

    // Configure allowed methods
    let methods: Vec<Method> = security_config
        .cors
        .allowed_methods
        .iter()
        .filter_map(|method| method.parse().ok())
        .collect();
    cors = cors.allow_methods(methods);

    // Configure allowed headers
    let headers: Vec<header::HeaderName> = security_config
        .cors
        .allowed_headers
        .iter()
        .filter_map(|header| header.parse().ok())
        .collect();
    cors = cors.allow_headers(headers);

    // Configure exposed headers
    let expose_headers: Vec<header::HeaderName> = security_config
        .cors
        .expose_headers
        .iter()
        .filter_map(|header| header.parse().ok())
        .collect();
    cors = cors.expose_headers(expose_headers);

    // Configure max age
    cors = cors.max_age(Duration::from_secs(security_config.cors.max_age));

    // Only allow credentials if not using wildcard origin
    // CORS spec doesn't allow credentials with wildcard origin
    if !allow_any_origin {
        cors = cors.allow_credentials(true);
    }

    cors
}
