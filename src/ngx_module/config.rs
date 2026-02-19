use ngx::core::NgxStr;
use ngx::ffi::ngx_str_t;
use rust_decimal::Decimal;
use std::time::Duration;

use crate::ngx_module::error::{ConfigError, Result};

/// Raw configuration from nginx directives.
///
/// Uses #[repr(C)] because nginx allocates this via ngx_pcalloc and
/// accesses fields via raw pointers.
#[repr(C)]
#[derive(Clone)]
pub struct X402Config {
    pub enabled: i64,
    pub amount_str: ngx_str_t,
    pub pay_to_str: ngx_str_t,
    pub facilitator_url_str: ngx_str_t,
    pub description_str: ngx_str_t,
    pub network_str: ngx_str_t,
    pub network_id_str: ngx_str_t,
    pub resource_str: ngx_str_t,
    pub asset_str: ngx_str_t,
    pub asset_decimals_str: ngx_str_t,
    pub timeout_str: ngx_str_t,
    pub facilitator_fallback_str: ngx_str_t,
    pub ttl_str: ngx_str_t,
    pub redis_url_str: ngx_str_t,
    pub replay_ttl_str: ngx_str_t,
}

impl Default for X402Config {
    fn default() -> Self {
        Self {
            enabled: 0,
            amount_str: ngx_str_t::default(),
            pay_to_str: ngx_str_t::default(),
            facilitator_url_str: ngx_str_t::default(),
            description_str: ngx_str_t::default(),
            network_str: ngx_str_t::default(),
            network_id_str: ngx_str_t::default(),
            resource_str: ngx_str_t::default(),
            asset_str: ngx_str_t::default(),
            asset_decimals_str: ngx_str_t::default(),
            timeout_str: ngx_str_t::default(),
            facilitator_fallback_str: ngx_str_t::default(),
            ttl_str: ngx_str_t::default(),
            redis_url_str: ngx_str_t::default(),
            replay_ttl_str: ngx_str_t::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FacilitatorFallback {
    Error,
    Pass,
}

pub struct ParsedX402Config {
    pub enabled: bool,
    pub amount: Option<Decimal>,
    pub pay_to: Option<String>,
    pub facilitator_url: Option<String>,
    pub description: Option<String>,
    pub network: Option<String>,
    pub network_id: Option<u64>,
    pub resource: Option<String>,
    pub asset: Option<String>,
    pub asset_decimals: Option<u8>,
    pub timeout: Option<Duration>,
    pub facilitator_fallback: FacilitatorFallback,
    pub ttl: Option<u32>,
    pub redis_url: Option<String>,
    pub replay_ttl: Option<u64>,
}

fn parse_ngx_str(s: ngx_str_t) -> Result<Option<String>> {
    if s.len == 0 {
        return Ok(None);
    }
    let ngx_str = unsafe { NgxStr::from_ngx_str(s) };
    let val = ngx_str
        .to_str()
        .map_err(|_| ConfigError::new("Invalid UTF-8 in config string"))?;
    Ok(Some(val.to_string()))
}

impl X402Config {
    pub fn parse(&self) -> Result<ParsedX402Config> {
        let amount = if let Some(s) = parse_ngx_str(self.amount_str)? {
            let amount = crate::config::validation::parse_amount(&s)
                .map_err(ConfigError::new)?;
            crate::config::validation::validate_amount(amount)
                .map_err(ConfigError::new)?;
            Some(amount)
        } else {
            None
        };

        let pay_to = if let Some(s) = parse_ngx_str(self.pay_to_str)? {
            crate::config::validation::validate_ethereum_address(&s)
                .map_err(ConfigError::new)?;
            Some(s)
        } else {
            None
        };

        let facilitator_url = if let Some(s) = parse_ngx_str(self.facilitator_url_str)? {
            crate::config::validation::validate_url(&s)
                .map_err(ConfigError::new)?;
            Some(s)
        } else {
            None
        };

        let description = parse_ngx_str(self.description_str)?;

        let network_id = if let Some(s) = parse_ngx_str(self.network_id_str)? {
            let id = s
                .parse::<u64>()
                .map_err(|e| ConfigError::new(format!("Invalid network_id: {e}")))?;
            crate::config::validation::chain_id_to_network(id)
                .map_err(ConfigError::new)?;
            Some(id)
        } else {
            None
        };

        let network = if network_id.is_some() {
            None
        } else if let Some(s) = parse_ngx_str(self.network_str)? {
            crate::config::validation::validate_network(&s)
                .map_err(ConfigError::new)?;
            Some(s)
        } else {
            None
        };

        let resource = parse_ngx_str(self.resource_str)?;

        let asset = if let Some(s) = parse_ngx_str(self.asset_str)? {
            crate::config::validation::validate_ethereum_address(&s)
                .map_err(ConfigError::new)?;
            Some(s)
        } else {
            None
        };

        let asset_decimals = if let Some(s) = parse_ngx_str(self.asset_decimals_str)? {
            let d = s
                .parse::<u8>()
                .map_err(|e| ConfigError::new(format!("Invalid asset_decimals: {e}")))?;
            if d > 28 {
                return Err(ConfigError::new("asset_decimals must be at most 28"));
            }
            Some(d)
        } else {
            None
        };

        let timeout = if let Some(s) = parse_ngx_str(self.timeout_str)? {
            let secs = s
                .parse::<u64>()
                .map_err(|e| ConfigError::new(format!("Invalid timeout: {e}")))?;
            if !(1..=300).contains(&secs) {
                return Err(ConfigError::new(
                    "Timeout must be between 1 and 300 seconds",
                ));
            }
            Some(Duration::from_secs(secs))
        } else {
            None
        };

        let facilitator_fallback =
            if let Some(s) = parse_ngx_str(self.facilitator_fallback_str)? {
                match s.to_lowercase().as_str() {
                    "error" | "500" => FacilitatorFallback::Error,
                    "pass" | "bypass" | "through" => FacilitatorFallback::Pass,
                    _ => {
                        return Err(ConfigError::new(
                            "facilitator_fallback must be 'error' or 'pass'",
                        ))
                    }
                }
            } else {
                FacilitatorFallback::Error
            };

        let ttl = if let Some(s) = parse_ngx_str(self.ttl_str)? {
            let val = s
                .parse::<u32>()
                .map_err(|e| ConfigError::new(format!("Invalid ttl: {e}")))?;
            if !(1..=3600).contains(&val) {
                return Err(ConfigError::new("ttl must be between 1 and 3600 seconds"));
            }
            Some(val)
        } else {
            None
        };

        let redis_url = if let Some(s) = parse_ngx_str(self.redis_url_str)? {
            Some(s)
        } else {
            std::env::var("X402_REDIS_URL").ok()
        };

        let replay_ttl = if let Some(s) = parse_ngx_str(self.replay_ttl_str)? {
            Some(
                s.parse::<u64>()
                    .map_err(|e| ConfigError::new(format!("Invalid replay_ttl: {e}")))?,
            )
        } else {
            None
        };

        Ok(ParsedX402Config {
            enabled: self.enabled != 0,
            amount,
            pay_to,
            facilitator_url,
            description,
            network,
            network_id,
            resource,
            asset,
            asset_decimals,
            timeout,
            facilitator_fallback,
            ttl,
            redis_url,
            replay_ttl,
        })
    }
}
