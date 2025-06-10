// use config::{Config, ConfigError, Environment, File};
use anyhow::Result;
use serde::Deserialize;
use std::{env, fs};

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseConfig {
    pub url: Option<String>,
    pub max_connections: u32,
    pub min_connections: u32,
    pub connect_timeout: u64,
    pub idle_timeout: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RedisConfig {
    pub url: Option<String>,
    pub pool_size: u32,
    pub timeout: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AuthConfig {
    pub jwt_secret: Option<String>,
    pub token_expiry: i64,
    pub bcrypt_cost: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: usize,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SecurityConfig {
    pub rate_limit: RateLimitConfig,
    pub cors: CorsConfig,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RateLimitConfig {
    pub requests_per_minute: u64,
    pub burst_size: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct CorsConfig {
    pub allowed_origins: Vec<String>,
    pub allowed_methods: Vec<String>,
    pub allowed_headers: Vec<String>,
    pub expose_headers: Vec<String>,
    pub max_age: u64,
}

#[derive(Debug, Deserialize, Clone)]
pub struct LoggingConfig {
    pub level: String,
    pub format: String,
    pub enable_console: bool,
    pub enable_file: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct PaginationConfig {
    pub default_limit: u32,
    pub max_limit: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AppConfig {
    pub database: DatabaseConfig,
    pub redis: RedisConfig,
    pub auth: AuthConfig,
    pub server: ServerConfig,
    pub security: SecurityConfig,
    pub logging: LoggingConfig,
    pub pagination: PaginationConfig,
    pub environment: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SecretConfig {
    pub database: DatabaseSecrets,
    pub redis: RedisSecrets,
    pub auth: AuthSecrets,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DatabaseSecrets {
    pub url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct RedisSecrets {
    pub url: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AuthSecrets {
    pub jwt_secret: String,
    pub refresh_secret: String,
}

impl AppConfig {
    pub fn from_yaml() -> Result<(Self, SecretConfig), anyhow::Error> {
        // Load main configuration from .config.yaml
        let config_content = fs::read_to_string(".config.yaml")
            .map_err(|e| anyhow::anyhow!("Failed to read .config.yaml: {}", e))?;

        let mut app_config: AppConfig = serde_yaml::from_str(&config_content)
            .map_err(|e| anyhow::anyhow!("Failed to parse .config.yaml: {}", e))?;

        // Load secrets from .secret.yaml
        let secret_content = fs::read_to_string(".secret.yaml")
            .map_err(|e| anyhow::anyhow!("Failed to read .secret.yaml: {}", e))?;

        let secret_config: SecretConfig = serde_yaml::from_str(&secret_content)
            .map_err(|e| anyhow::anyhow!("Failed to parse .secret.yaml: {}", e))?;

        // Override with environment variables if present
        if let Ok(env_val) = env::var("ENVIRONMENT") {
            app_config.environment = env_val;
        }

        if let Ok(host) = env::var("HOST") {
            app_config.server.host = host;
        }

        if let Ok(port) = env::var("PORT") {
            app_config.server.port = port
                .parse()
                .map_err(|e| anyhow::anyhow!("Invalid PORT value: {}", e))?;
        }

        if let Ok(log_level) = env::var("LOG_LEVEL") {
            app_config.logging.level = log_level;
        }

        // Apply secrets to config
        app_config.database.url = Some(secret_config.database.url.clone());
        app_config.redis.url = Some(secret_config.redis.url.clone());
        app_config.auth.jwt_secret = Some(secret_config.auth.jwt_secret.clone());

        Ok((app_config, secret_config))
    }

    pub fn is_production(&self) -> bool {
        self.environment == "production"
    }

    pub fn is_development(&self) -> bool {
        self.environment == "development"
    }

    pub fn get_database_url(&self) -> Result<&str, anyhow::Error> {
        self.database
            .url
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Database URL not configured"))
            .map(|s| s.as_str())
    }

    pub fn get_redis_url(&self) -> Result<&str, anyhow::Error> {
        self.redis
            .url
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("Redis URL not configured"))
            .map(|s| s.as_str())
    }

    pub fn get_jwt_secret(&self) -> Result<&str, anyhow::Error> {
        self.auth
            .jwt_secret
            .as_ref()
            .ok_or_else(|| anyhow::anyhow!("JWT secret not configured"))
            .map(|s| s.as_str())
    }
}
