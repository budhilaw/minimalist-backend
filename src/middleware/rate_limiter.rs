use anyhow::Result;
use chrono::{DateTime, Utc};
use redis::{aio::ConnectionManager, Client};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthRateLimitInfo {
    pub allowed: bool,
    pub remaining_attempts: u32,
    pub reset_time: DateTime<Utc>,
    pub lockout_seconds: Option<u64>,
    pub reason: Option<String>,
    pub is_permanently_blocked: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlockedIpInfo {
    pub ip: String,
    pub blocked_at: DateTime<Utc>,
    pub reason: String,
    pub attempt_count: u32,
    pub expires_at: Option<DateTime<Utc>>, // None = permanent
}

#[derive(Clone)]
pub struct RedisRateLimiter {
    client: Client,

    // Authentication rate limiting
    auth_ip_limit: u32,
    auth_ip_window_seconds: u64,
    auth_user_limit: u32,
    auth_user_window_seconds: u64,

    // IP blocking thresholds
    ip_block_threshold: u32,      // Block IP after this many failures
    ip_block_duration_hours: u64, // Block duration (0 = permanent)

    // General API rate limiting
    #[allow(dead_code)]
    api_limit: u32,
    #[allow(dead_code)]
    api_window_seconds: u64,
}

impl RedisRateLimiter {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        redis_url: &str,
        auth_ip_limit: u32,
        auth_ip_window_seconds: u64,
        auth_user_limit: u32,
        auth_user_window_seconds: u64,
        ip_block_threshold: u32,
        ip_block_duration_hours: u64,
        api_limit: u32,
        api_window_seconds: u64,
    ) -> Result<Self> {
        let client = Client::open(redis_url)?;

        Ok(Self {
            client,
            auth_ip_limit,
            auth_ip_window_seconds,
            auth_user_limit,
            auth_user_window_seconds,
            ip_block_threshold,
            ip_block_duration_hours,
            api_limit,
            api_window_seconds,
        })
    }

    pub async fn get_connection(&self) -> Result<ConnectionManager> {
        Ok(ConnectionManager::new(self.client.clone()).await?)
    }

    pub async fn check_auth_rate_limit(
        &self,
        ip: &str,
        username: Option<&str>,
    ) -> Result<(bool, AuthRateLimitInfo)> {
        let mut conn = self.get_connection().await?;

        // First check if IP is blocked
        let blocked_key = format!("blocked_ip:{}", ip);
        let is_blocked: Option<String> = redis::cmd("GET")
            .arg(&blocked_key)
            .query_async(&mut conn)
            .await?;

        if is_blocked.is_some() {
            return Ok((
                false,
                AuthRateLimitInfo {
                    allowed: false,
                    remaining_attempts: 0,
                    reset_time: Utc::now(),
                    lockout_seconds: None,
                    reason: Some("IP address is blocked due to suspicious activity".to_string()),
                    is_permanently_blocked: true,
                },
            ));
        }

        let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();

        // Check IP-based rate limiting
        let ip_key = format!("auth_rate_limit:ip:{}", ip);
        let cutoff_time = now - self.auth_ip_window_seconds;

        // Remove expired entries and count current attempts
        redis::cmd("ZREMRANGEBYSCORE")
            .arg(&ip_key)
            .arg(0)
            .arg(cutoff_time)
            .query_async::<()>(&mut conn)
            .await?;

        let ip_count: u32 = redis::cmd("ZCARD")
            .arg(&ip_key)
            .query_async(&mut conn)
            .await?;

        // Check username-based rate limiting if provided
        let user_count = if let Some(user) = username {
            let user_key = format!("auth_rate_limit:user:{}", user);

            redis::cmd("ZREMRANGEBYSCORE")
                .arg(&user_key)
                .arg(0)
                .arg(cutoff_time)
                .query_async::<()>(&mut conn)
                .await?;

            redis::cmd("ZCARD")
                .arg(&user_key)
                .query_async::<u32>(&mut conn)
                .await?
        } else {
            0
        };

        // Check if limits exceeded
        let ip_exceeded = ip_count >= self.auth_ip_limit;
        let user_exceeded = user_count >= self.auth_user_limit;

        if ip_exceeded || user_exceeded {
            let remaining_time = self.auth_ip_window_seconds;
            let reset_time = Utc::now() + chrono::Duration::seconds(remaining_time as i64);

            let reason = if ip_exceeded && user_exceeded {
                format!(
                    "Too many login attempts from this IP ({}/{}) and for this user ({}/{})",
                    ip_count, self.auth_ip_limit, user_count, self.auth_user_limit
                )
            } else if ip_exceeded {
                format!(
                    "Too many login attempts from this IP ({}/{})",
                    ip_count, self.auth_ip_limit
                )
            } else {
                format!(
                    "Too many login attempts for this user ({}/{})",
                    user_count, self.auth_user_limit
                )
            };

            return Ok((
                false,
                AuthRateLimitInfo {
                    allowed: false,
                    remaining_attempts: 0,
                    reset_time,
                    lockout_seconds: Some(remaining_time),
                    reason: Some(reason),
                    is_permanently_blocked: false,
                },
            ));
        }

        // Calculate remaining attempts
        let ip_remaining = self.auth_ip_limit.saturating_sub(ip_count);
        let user_remaining = if username.is_some() {
            self.auth_user_limit.saturating_sub(user_count)
        } else {
            self.auth_user_limit
        };

        let remaining_attempts = ip_remaining.min(user_remaining);
        let reset_time = Utc::now() + chrono::Duration::seconds(self.auth_ip_window_seconds as i64);

        Ok((
            true,
            AuthRateLimitInfo {
                allowed: true,
                remaining_attempts,
                reset_time,
                lockout_seconds: None,
                reason: None,
                is_permanently_blocked: false,
            },
        ))
    }

    // Block an IP address manually
    pub async fn block_ip(&self, ip: &str, reason: &str, permanent: bool) -> Result<()> {
        let mut conn = self.get_connection().await?;

        // Get current attempt count
        let ip_key = format!("auth_rate_limit:ip:{}", ip);
        let attempt_count: u32 = redis::cmd("ZCARD")
            .arg(&ip_key)
            .query_async(&mut conn)
            .await
            .unwrap_or(0);

        let blocked_info = BlockedIpInfo {
            ip: ip.to_string(),
            blocked_at: Utc::now(),
            reason: reason.to_string(),
            attempt_count,
            expires_at: if permanent || self.ip_block_duration_hours == 0 {
                None // Permanent block
            } else {
                Some(Utc::now() + chrono::Duration::hours(self.ip_block_duration_hours as i64))
            },
        };

        let blocked_key = format!("blocked_ip:{}", ip);
        let serialized = serde_json::to_string(&blocked_info)?;

        if permanent || self.ip_block_duration_hours == 0 {
            // Permanent block
            redis::cmd("SET")
                .arg(&blocked_key)
                .arg(&serialized)
                .query_async::<()>(&mut conn)
                .await?;
        } else {
            // Temporary block with TTL
            redis::cmd("SETEX")
                .arg(&blocked_key)
                .arg(self.ip_block_duration_hours * 3600) // Convert to seconds
                .arg(&serialized)
                .query_async::<()>(&mut conn)
                .await?;
        }

        tracing::warn!(
            "IP {} blocked. Reason: {}. Attempts: {}. Permanent: {}",
            ip,
            reason,
            attempt_count,
            permanent || self.ip_block_duration_hours == 0
        );

        Ok(())
    }

    // Unblock an IP address
    pub async fn unblock_ip(&self, ip: &str) -> Result<()> {
        let mut conn = self.get_connection().await?;

        let blocked_key = format!("blocked_ip:{}", ip);
        redis::cmd("DEL")
            .arg(&blocked_key)
            .query_async::<()>(&mut conn)
            .await?;

        tracing::info!("IP {} unblocked", ip);
        Ok(())
    }

    // Get all blocked IPs (simplified version)
    pub async fn get_blocked_ips(&self) -> Result<Vec<BlockedIpInfo>> {
        let _conn = self.get_connection().await?;

        // This is a simplified version - in production you'd want to scan for blocked_ip:* keys
        // For now, we'll return empty list and you can manually track blocked IPs
        Ok(vec![])
    }
}

// Record authentication failure
pub async fn record_auth_failure(
    limiter: &RedisRateLimiter,
    identifier: &str,
    username: &str,
) -> Result<()> {
    let mut conn = limiter.get_connection().await?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();

    let ip_key = format!("auth_rate_limit:ip:{}", identifier);
    let user_key = format!("auth_rate_limit:user:{}", username);
    let attempt_id = format!("{}:{}", now, uuid::Uuid::new_v4());

    // Record failed attempt for both IP and username
    redis::cmd("ZADD")
        .arg(&ip_key)
        .arg(now as f64)
        .arg(&attempt_id)
        .query_async::<()>(&mut conn)
        .await?;

    redis::cmd("ZADD")
        .arg(&user_key)
        .arg(now as f64)
        .arg(&attempt_id)
        .query_async::<()>(&mut conn)
        .await?;

    // Set TTL for cleanup
    redis::cmd("EXPIRE")
        .arg(&ip_key)
        .arg(limiter.auth_ip_window_seconds)
        .query_async::<()>(&mut conn)
        .await?;

    redis::cmd("EXPIRE")
        .arg(&user_key)
        .arg(limiter.auth_user_window_seconds)
        .query_async::<()>(&mut conn)
        .await?;

    // Check if IP should be auto-blocked
    check_and_auto_block_ip(limiter, identifier).await?;

    Ok(())
}

// Clear rate limiting after successful authentication
pub async fn clear_auth_rate_limit(
    limiter: &RedisRateLimiter,
    identifier: &str,
    username: &str,
) -> Result<()> {
    let mut conn = limiter.get_connection().await?;

    let ip_key = format!("auth_rate_limit:ip:{}", identifier);
    let user_key = format!("auth_rate_limit:user:{}", username);

    // Clear both IP and username-based rate limiting
    redis::cmd("DEL")
        .arg(&ip_key)
        .query_async::<()>(&mut conn)
        .await?;

    redis::cmd("DEL")
        .arg(&user_key)
        .query_async::<()>(&mut conn)
        .await?;

    Ok(())
}

// Simple function to check if IP should be auto-blocked
pub async fn check_and_auto_block_ip(limiter: &RedisRateLimiter, ip: &str) -> Result<()> {
    let mut conn = limiter.get_connection().await?;

    let ip_key = format!("auth_rate_limit:ip:{}", ip);
    let attempt_count: u32 = redis::cmd("ZCARD")
        .arg(&ip_key)
        .query_async(&mut conn)
        .await
        .unwrap_or(0);

    // Auto-block if more than 20 failed attempts from same IP
    if attempt_count >= limiter.ip_block_threshold {
        let reason = format!("Auto-blocked after {} failed login attempts", attempt_count);
        limiter.block_ip(ip, &reason, false).await?;
    }

    Ok(())
}
