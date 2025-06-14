use portfolio_backend::{
    database::{connection::create_pool, seeder::DatabaseSeeder},
    utils::config::AppConfig,
};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "portfolio_backend=info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Load configuration
    let (config, _secret_config) = AppConfig::from_yaml()?;

    // Create database connection pool
    let database_url = config.get_database_url()?;
    let pool = create_pool(database_url, &config.database).await?;

    // Run seeder
    let seeder = DatabaseSeeder::new(pool);
    seeder.seed_all().await?;

    println!("✅ Database seeding completed successfully!");

    Ok(())
} 