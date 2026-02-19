use std::fmt;

#[derive(Debug)]
pub struct ConfigError {
    message: String,
}

impl ConfigError {
    pub fn new(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
        }
    }
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for ConfigError {}

impl From<&str> for ConfigError {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for ConfigError {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

pub type Result<T> = std::result::Result<T, ConfigError>;

pub mod user_errors {
    pub const INVALID_PAYMENT: &str = "Invalid payment payload";
    pub const PAYMENT_VERIFICATION_FAILED: &str = "Payment verification failed";
    pub const CONFIGURATION_ERROR: &str = "Server configuration error";
    pub const TIMEOUT: &str = "Payment verification timed out";
    pub const REPLAY_DETECTED: &str = "Payment replay detected";
}
