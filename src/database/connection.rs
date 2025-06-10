use crate::utils::{config::DatabaseConfig, errors::AppError};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::time::Duration;

pub async fn create_pool(database_url: &str, config: &DatabaseConfig) -> Result<PgPool, AppError> {
    let pool = PgPoolOptions::new()
        .max_connections(config.max_connections)
        .min_connections(config.min_connections)
        .acquire_timeout(Duration::from_secs(config.connect_timeout))
        .idle_timeout(Duration::from_secs(config.idle_timeout))
        .test_before_acquire(true)
        .connect(database_url)
        .await?;

    tracing::info!("Database connection pool created successfully");

    Ok(pool)
}

pub async fn run_migrations(pool: &PgPool) -> Result<(), AppError> {
    sqlx::migrate!("./migrations").run(pool).await?;

    tracing::info!("Database migrations completed successfully");

    Ok(())
}
