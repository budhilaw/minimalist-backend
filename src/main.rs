use axum::{
    middleware,
    routing::{delete, get, post, put},
    Router,
};
use std::{env, net::SocketAddr, sync::Arc};
use tokio::signal;
use tower::ServiceBuilder;
use tower_http::{compression::CompressionLayer, trace::TraceLayer};
use tracing::info;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use portfolio_backend::{
    database::{
        connection::{create_pool, run_migrations},
        // seeder::DatabaseSeeder, // Removed unused import - seeding disabled to prevent data loss
    },
    handlers::{
        admin_settings, audit_log, auth, comment, portfolio, post, service, user_notification,
    },
    middleware::{
        auth::auth_middleware,
        rate_limiter::RedisRateLimiter,
        security::{
            create_cors_layer, create_rate_limiter, logging_middleware, request_id_middleware,
            security_headers_middleware,
        },
    },
    repositories::{
        comment_repository::CommentRepository, portfolio_repository::PortfolioRepository,
        post_repository::PostRepository, service_repository::ServiceRepository,
        user_repository::UserRepository, AdminSettingsRepository, AuditLogRepository,
        UserNotificationRepository,
    },
    services::{
        admin_settings_service::{AdminSettingsService, AdminSettingsServiceTrait},
        audit_log_service::{AuditLogService, AuditLogServiceTrait},
        auth_service::AuthService,
        blog_service::{BlogService, BlogServiceTrait},
        comment_service::{CommentService, CommentServiceTrait},
        portfolio_service::{PortfolioService, PortfolioServiceTrait},
        service_service::{ServiceService, ServiceServiceTrait},
        user_notification_service::{UserNotificationService, UserNotificationServiceTrait},
    },
    utils::{config::AppConfig, errors::AppError},
};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Load configuration
    let (config, _secret_config) = AppConfig::from_yaml()?;

    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "portfolio_backend=debug,tower_http=debug".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("Starting portfolio backend server...");

    // Create database connection pool
    let database_url = config.get_database_url()?;
    let pool = create_pool(database_url, &config.database).await?;

    // Run database migrations
    run_migrations(&pool).await?;

    // DISABLED: Automatic seeding to prevent data loss
    //
    // WARNING: The automatic seeding was causing data loss on every restart
    // because it drops all tables and re-seeds with sample data.
    //
    // To manually seed the database when needed, use:
    // let seeder = DatabaseSeeder::new(pool.clone());
    // seeder.seed_all().await?;
    //
    // Seed database in development
    // if config.is_development() {
    //     let seeder = DatabaseSeeder::new(pool.clone());
    //     seeder.seed_all().await?;
    //     info!("Database seeded successfully");
    // }

    // Initialize repositories
    let user_repository = Arc::new(UserRepository::new(pool.clone()));
    let portfolio_repository = Arc::new(PortfolioRepository::new(pool.clone()));
    let service_repository = Arc::new(ServiceRepository::new(pool.clone()));
    let post_repository = Arc::new(PostRepository::new(pool.clone()));
    let comment_repository = Arc::new(CommentRepository::new(pool.clone()));
    let audit_log_repository = Arc::new(AuditLogRepository::new(pool.clone()));
    let admin_settings_repository = Arc::new(AdminSettingsRepository::new(pool.clone()));
    let user_notification_repository: Arc<UserNotificationRepository> =
        Arc::new(UserNotificationRepository::new(pool.clone()));

    // Safely initialize admin settings if they don't exist (won't overwrite existing data)
    admin_settings_repository.ensure_settings_exist().await?;

    // Initialize services
    let auth_service = AuthService::new(
        user_repository.clone(),
        config.get_jwt_secret()?.to_string(),
        config.auth.token_expiry,
    );

    let portfolio_service: Arc<dyn PortfolioServiceTrait> =
        Arc::new(PortfolioService::new(portfolio_repository));
    let service_service: Arc<dyn ServiceServiceTrait> =
        Arc::new(ServiceService::new(service_repository));
    let blog_service: Arc<dyn BlogServiceTrait> = Arc::new(BlogService::new(post_repository));
    let audit_log_service: Arc<dyn AuditLogServiceTrait> =
        Arc::new(AuditLogService::new(audit_log_repository));
    let admin_settings_service: Arc<dyn AdminSettingsServiceTrait> =
        Arc::new(AdminSettingsService::new(admin_settings_repository));
    let comment_service: Arc<dyn CommentServiceTrait> = Arc::new(CommentService::new(
        comment_repository,
        admin_settings_service.clone(),
    ));
    let user_notification_service: Arc<dyn UserNotificationServiceTrait> =
        Arc::new(UserNotificationService::new(user_notification_repository));

    // CAPTCHA verifier and spam detector removed since contact form is no longer used

    // Initialize Redis rate limiter
    let rate_limiter = match config.get_redis_url() {
        Ok(redis_url) => match create_rate_limiter(&config.security, redis_url).await {
            Ok(limiter) => {
                info!("Redis rate limiter initialized successfully");
                Some(limiter)
            }
            Err(e) => {
                tracing::warn!("Failed to initialize Redis rate limiter: {}", e);
                None
            }
        },
        Err(e) => {
            tracing::warn!("Redis URL not configured: {}", e);
            None
        }
    };

    // Initialize handler states (dependency injection)
    let portfolio_state = portfolio::PortfolioState { portfolio_service };
    let service_state = service::ServiceState { service_service };
    let post_state = post::PostState { blog_service };
    let comment_state = comment::CommentState { comment_service };
    let audit_log_state = audit_log::AuditLogState {
        audit_log_service: audit_log_service.clone(),
    };
    let admin_settings_state = admin_settings::AdminSettingsState {
        admin_settings_service: admin_settings_service.clone(),
        rate_limiter: rate_limiter.clone(),
    };
    let user_notification_state = user_notification::UserNotificationState {
        user_notification_service,
    };

    // Create auth state with auth service, audit log service, and rate limiter
    let auth_state = auth::AuthState {
        auth_service: auth_service.clone(),
        audit_log_service,
        rate_limiter: rate_limiter.clone(),
    };

    // Build our application with routes
    let app = create_app(
        auth_state,
        portfolio_state,
        service_state,
        post_state,
        comment_state,
        audit_log_state,
        admin_settings_state,
        user_notification_state,
        &config,
        rate_limiter,
    );

    // Create server address
    let addr = SocketAddr::from(([0, 0, 0, 0], config.server.port));
    info!("Starting server on {}", addr);

    // Create server
    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn create_app(
    auth_state: auth::AuthState,
    portfolio_state: portfolio::PortfolioState,
    service_state: service::ServiceState,
    post_state: post::PostState,
    comment_state: comment::CommentState,
    audit_log_state: audit_log::AuditLogState,
    admin_settings_state: admin_settings::AdminSettingsState,
    user_notification_state: user_notification::UserNotificationState,
    config: &AppConfig,
    _rate_limiter: Option<Arc<RedisRateLimiter>>,
) -> Router {
    // Create CORS layer with configuration
    let cors = create_cors_layer(&config.security);

    // Create protected routes that require authentication
    let protected_routes = Router::new()
        .route("/me", get(auth::me))
        .route("/refresh", post(auth::refresh_token))
        .route("/profile", put(auth::update_profile))
        .route("/change-password", put(auth::change_password))
        .route("/logout", post(auth::logout))
        .with_state(auth_state.clone())
        .route_layer(middleware::from_fn_with_state(
            auth_state.auth_service.clone(),
            auth_middleware,
        ));

    // Create public routes
    let public_routes = Router::new()
        .route("/login", post(auth::login))
        .route("/health", get(health_check))
        .with_state(auth_state.clone());

    // Portfolio routes (protected)
    let portfolio_routes = Router::new()
        .route(
            "/",
            get(portfolio::get_all_projects).post(portfolio::create_project),
        )
        .route(
            "/:id",
            get(portfolio::get_project)
                .put(portfolio::update_project)
                .delete(portfolio::delete_project),
        )
        .route("/featured", get(portfolio::get_featured_projects))
        .route("/stats", get(portfolio::get_portfolio_stats))
        .route("/:id/featured", put(portfolio::update_featured_status))
        .with_state(portfolio_state.clone())
        .route_layer(middleware::from_fn_with_state(
            auth_state.auth_service.clone(),
            auth_middleware,
        ));

    // Portfolio public routes (no authentication required)
    let portfolio_public_routes = Router::new()
        .route("/", get(portfolio::get_all_projects))
        .route("/:id", get(portfolio::get_project))
        .route("/slug/:slug", get(portfolio::get_project_by_slug))
        .route("/featured", get(portfolio::get_featured_projects))
        .with_state(portfolio_state);

    // Service routes (protected)
    let service_routes = Router::new()
        .route(
            "/",
            get(service::get_all_services).post(service::create_service),
        )
        .route(
            "/:id",
            get(service::get_service)
                .put(service::update_service)
                .delete(service::delete_service),
        )
        .route("/stats", get(service::get_service_stats))
        .route("/:id/activate", put(service::update_service_status))
        .with_state(service_state.clone())
        .route_layer(middleware::from_fn_with_state(
            auth_state.auth_service.clone(),
            auth_middleware,
        ));

    // Service public routes (no authentication required)
    let service_public_routes = Router::new()
        .route("/", get(service::get_all_services))
        .route("/active", get(service::get_active_services))
        .route("/:id", get(service::get_service))
        .with_state(service_state);

    // Post routes (protected for admin)
    let post_protected_routes = Router::new()
        .route("/", post(post::create_post))
        .route("/:id", put(post::update_post).delete(post::delete_post))
        .route("/:id/publish", put(post::update_published_status))
        .route("/stats", get(post::get_post_stats))
        .with_state(post_state.clone())
        .route_layer(middleware::from_fn_with_state(
            auth_state.auth_service.clone(),
            auth_middleware,
        ));

    // Post public routes (no authentication required)
    let post_public_routes = Router::new()
        .route("/", get(post::get_all_posts))
        .route("/:id", get(post::get_post))
        .route("/slug/:slug", get(post::get_post_by_slug))
        .route("/published", get(post::get_published_posts))
        .route("/featured", get(post::get_featured_posts))
        .route("/categories", get(post::get_all_posts))
        .with_state(post_state);

    // Comment routes (protected for admin)
    let comment_protected_routes = Router::new()
        .route("/", get(comment::get_all_comments))
        .route(
            "/:id",
            get(comment::get_comment).delete(comment::delete_comment),
        )
        .route("/:id/status", put(comment::update_comment_status))
        .route("/:id/approve", put(comment::approve_comment))
        .route("/:id/reject", put(comment::reject_comment))
        .route("/pending", get(comment::get_pending_comments))
        .route("/bulk-status", put(comment::bulk_update_comment_status))
        .route("/stats", get(comment::get_comment_stats))
        .with_state(comment_state.clone())
        .route_layer(middleware::from_fn_with_state(
            auth_state.auth_service.clone(),
            auth_middleware,
        ));

    // Comment public routes (no authentication required)
    let comment_public_routes = Router::new()
        .route("/post/:post_id", get(comment::get_comments_by_post))
        .route("/", post(comment::create_comment))
        .with_state(comment_state);

    // Audit log routes (protected)
    let audit_log_routes = Router::new()
        .route(
            "/",
            get(audit_log::get_audit_logs).post(audit_log::create_audit_log).delete(audit_log::delete_all_audit_logs),
        )
        .route("/:id", get(audit_log::get_audit_log))
        .route("/recent", get(audit_log::get_recent_audit_logs))
        .route("/stats", get(audit_log::get_audit_log_stats))
        .with_state(audit_log_state)
        .route_layer(middleware::from_fn_with_state(
            auth_state.auth_service.clone(),
            auth_middleware,
        ));

    // Admin settings routes (protected)
    let admin_settings_routes = Router::new()
        .route(
            "/",
            get(admin_settings::get_settings).put(admin_settings::update_settings),
        )
        .route("/features", get(admin_settings::get_settings))
        .route("/features", put(admin_settings::update_feature_settings))
        .route("/security", get(admin_settings::get_settings))
        .route("/security", put(admin_settings::update_security_settings))
        .route(
            "/security/blocked-ips",
            get(admin_settings::get_blocked_ips),
        )
        .route("/security/block-ip", post(admin_settings::block_ip))
        .route(
            "/security/blocked-ips/:ip",
            delete(admin_settings::unblock_ip),
        )
        .route("/security/stats", get(admin_settings::get_security_stats))
        .route("/reset", post(admin_settings::reset_settings))
        .route(
            "/features/:feature/enabled",
            get(admin_settings::is_feature_enabled),
        )
        .route(
            "/maintenance-mode",
            get(admin_settings::get_maintenance_mode),
        )
        .route(
            "/maintenance-mode",
            put(admin_settings::get_maintenance_mode),
        )
        .with_state(admin_settings_state.clone())
        .route_layer(middleware::from_fn_with_state(
            auth_state.auth_service.clone(),
            auth_middleware,
        ));

    // Public settings routes (no authentication required)
    let settings_public_routes = Router::new()
        .route("/public", get(admin_settings::get_public_settings))
        .with_state(admin_settings_state);

    // User notification routes (protected)
    let user_notification_routes = Router::new()
        .route("/", get(user_notification::get_user_notifications))
        .route(
            "/mark-read",
            post(user_notification::mark_notification_read),
        )
        .route(
            "/mark-multiple-read",
            post(user_notification::mark_notifications_read),
        )
        .route(
            "/mark-all-read",
            post(user_notification::mark_all_notifications_read),
        )
        .route("/stats", get(user_notification::get_notification_stats))
        .route("/unread-count", get(user_notification::get_unread_count))
        .route(
            "/preferences",
            get(user_notification::get_notification_preferences),
        )
        .route(
            "/preferences",
            put(user_notification::update_notification_preference),
        )
        .with_state(user_notification_state)
        .route_layer(middleware::from_fn_with_state(
            auth_state.auth_service.clone(),
            auth_middleware,
        ));

    Router::new()
        .nest("/api/v1/auth", protected_routes)
        .nest("/api/v1/auth", public_routes)
        .nest("/api/v1/portfolio", portfolio_routes)
        .nest("/api/v1/portfolio/public", portfolio_public_routes)
        .nest("/api/v1/services", service_routes)
        .nest("/api/v1/services/public", service_public_routes)
        .nest("/api/v1/posts", post_protected_routes)
        .nest("/api/v1/posts", post_public_routes)
        .nest("/api/v1/comments", comment_protected_routes)
        .nest("/api/v1/comments", comment_public_routes)
        .nest("/api/v1/admin/audit-logs", audit_log_routes)
        .nest("/api/v1/admin/settings", admin_settings_routes)
        .nest("/api/v1/settings", settings_public_routes)
        .nest("/api/v1/user/notifications", user_notification_routes)
        .route("/api/v1/health", get(health_check))
        .layer(
            ServiceBuilder::new()
                .layer(middleware::from_fn(security_headers_middleware))
                .layer(middleware::from_fn(request_id_middleware))
                .layer(middleware::from_fn(logging_middleware))
                .layer(TraceLayer::new_for_http())
                .layer(CompressionLayer::new())
                .layer(cors)
                .into_inner(),
        )
        .with_state(auth_state)
}

async fn health_check() -> Result<axum::Json<serde_json::Value>, AppError> {
    Ok(axum::Json(serde_json::json!({
        "status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "version": env!("CARGO_PKG_VERSION")
    })))
}

async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }

    info!("Received shutdown signal, starting graceful shutdown");
}
