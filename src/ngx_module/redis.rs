use crate::ngx_module::error::{ConfigError, Result};
use redis::Commands;
use sha2::{Digest, Sha256};
use std::sync::{Mutex, OnceLock};

static REDIS_CLIENT: OnceLock<Mutex<redis::Client>> = OnceLock::new();

pub fn init_redis(url: &str) -> Result<()> {
    match redis::Client::open(url) {
        Ok(client) => {
            if REDIS_CLIENT.set(Mutex::new(client)).is_err() {
                log::warn!("Redis client already initialized");
            } else {
                log::info!("Connected to Redis at {url}");
            }
            Ok(())
        }
        Err(e) => Err(ConfigError::new(format!(
            "Failed to create Redis client: {e}"
        ))),
    }
}

fn get_connection() -> Option<redis::Connection> {
    let client = REDIS_CLIENT.get()?;
    let guard = client.lock().ok()?;
    guard.get_connection().ok()
}

/// Get a dynamic price override from Redis for the given path.
/// Returns None if Redis is not configured or no override exists.
pub fn get_dynamic_price(path: &str) -> Option<String> {
    let mut conn = get_connection()?;
    conn.get(path).ok()
}

/// Check if a payment signature has been used before (replay prevention).
pub fn is_payment_used(payment_b64: &str) -> bool {
    let mut conn = match get_connection() {
        Some(c) => c,
        None => return false,
    };

    let hash = payment_hash(payment_b64);
    let key = format!("x402:payment_sig:{hash}");
    conn.exists::<_, bool>(&key).unwrap_or(false)
}

/// Store a payment signature as used with TTL.
pub fn store_payment_as_used(payment_b64: &str, ttl_seconds: u64) -> Result<()> {
    let mut conn = get_connection()
        .ok_or_else(|| ConfigError::new("Redis not configured"))?;

    let hash = payment_hash(payment_b64);
    let key = format!("x402:payment_sig:{hash}");
    conn.set_ex::<_, _, ()>(&key, "used", ttl_seconds as u64)
        .map_err(|e| ConfigError::new(format!("Failed to store payment in Redis: {e}")))?;

    log::debug!("Stored payment signature as used: {hash} (TTL: {ttl_seconds}s)");
    Ok(())
}

fn payment_hash(payment_b64: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(payment_b64.as_bytes());
    hex::encode(hasher.finalize())
}

pub fn is_redis_configured() -> bool {
    REDIS_CLIENT.get().is_some()
}
