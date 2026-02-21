use crate::config::validation::chain_id_to_network;
use crate::ngx_module::config::ParsedX402Config;
use crate::ngx_module::error::{ConfigError, Result};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

/// Payment requirements sent to the client in the 402 response.
///
/// Uses the x402 v2 format with CAIP-2 chain IDs.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PaymentRequirements {
    pub scheme: String,
    pub network: String,
    /// x402 v2: "amount" (required by client)
    pub amount: String,
    /// x402 v1: "maxAmountRequired"
    pub max_amount_required: String,
    pub resource: String,
    pub description: String,
    pub mime_type: Option<String>,
    pub pay_to: String,
    pub max_timeout_seconds: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asset: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extra: Option<serde_json::Value>,
}

/// 402 response body as per x402 protocol.
#[derive(Debug, Serialize, Deserialize)]
pub struct PaymentRequirementsResponse {
    #[serde(rename = "x402Version")]
    pub x402_version: u8,
    pub error: String,
    pub accepts: Vec<PaymentRequirements>,
}

impl PaymentRequirementsResponse {
    pub fn new(error: &str, accepts: Vec<PaymentRequirements>) -> Self {
        Self {
            x402_version: 2,
            error: error.to_string(),
            accepts,
        }
    }
}

fn amount_to_smallest_unit(amount: Decimal, decimals: u8) -> String {
    let multiplier = Decimal::from(10u64.pow(decimals as u32));
    (amount * multiplier).normalize().to_string()
}

fn resolve_network(config: &ParsedX402Config) -> Result<String> {
    if let Some(chain_id) = config.network_id {
        chain_id_to_network(chain_id).map_err(ConfigError::new)?;
        Ok(format!("eip155:{chain_id}"))
    } else if let Some(ref net) = config.network {
        if net.contains(':') {
            Ok(net.clone())
        } else {
            match net.as_str() {
                "base" => Ok("eip155:8453".to_string()),
                "base-sepolia" => Ok("eip155:84532".to_string()),
                "polygon" => Ok("eip155:137".to_string()),
                "polygon-amoy" => Ok("eip155:80002".to_string()),
                "avalanche" => Ok("eip155:43114".to_string()),
                "avalanche-fuji" => Ok("eip155:43113".to_string()),
                _ => Ok(net.clone()),
            }
        }
    } else {
        Ok("eip155:8453".to_string())
    }
}

fn default_usdc_address(network: &str) -> Option<&'static str> {
    match network {
        "eip155:8453" => Some("0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"),
        "eip155:84532" => Some("0x036CbD53842c5426634e7929541eC2318f3dCF7e"),
        "eip155:137" => Some("0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359"),
        _ => None,
    }
}

fn eip712_extra_for_asset(asset: &str) -> Option<serde_json::Value> {
    let normalized = asset.to_lowercase();
    if normalized == "0x036cbd53842c5426634e7929541ec2318f3dcf7e" {
        return Some(serde_json::json!({
            "name": "USDC",
            "version": "2"
        }));
    }
    let usdc_addrs = [
        "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913",
        "0x3c499c542cef5e3811e1192ce70d8cc03d5c3359",
    ];
    if usdc_addrs.contains(&normalized.as_str()) {
        Some(serde_json::json!({
            "name": "USD Coin",
            "version": "2"
        }))
    } else {
        None
    }
}

pub fn create_requirements(
    config: &ParsedX402Config,
    resource: &str,
    mime_type: Option<&str>,
) -> Result<PaymentRequirements> {
    let amount = config
        .amount
        .ok_or_else(|| ConfigError::new("Amount not configured"))?;
    if amount < Decimal::ZERO {
        return Err(ConfigError::new("Amount cannot be negative"));
    }
    let pay_to = config
        .pay_to
        .as_ref()
        .ok_or_else(|| ConfigError::new("pay_to address not configured"))?;
    let network = resolve_network(config)?;
    let decimals = config.asset_decimals.unwrap_or(6);
    let max_amount_required = amount_to_smallest_unit(amount, decimals);
    let asset_address = if let Some(ref custom) = config.asset {
        custom.clone()
    } else {
        default_usdc_address(&network)
            .map(|s| s.to_string())
            .unwrap_or_default()
    };
    let resource = crate::config::validation::validate_resource_path(resource)
        .map_err(|e| ConfigError::new(e))?;
    let max_timeout_seconds = config.ttl.unwrap_or(60);
    let mime = mime_type.unwrap_or("application/json");
    Ok(PaymentRequirements {
        scheme: "exact".to_string(),
        network,
        amount: max_amount_required.clone(),
        max_amount_required,
        resource,
        description: config.description.as_deref().unwrap_or("").to_string(),
        mime_type: Some(mime.to_string()),
        pay_to: pay_to.to_lowercase(),
        max_timeout_seconds,
        asset: if asset_address.is_empty() { None } else { Some(asset_address.clone()) },
        extra: if asset_address.is_empty() { None } else { eip712_extra_for_asset(&asset_address) },
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_amount_to_smallest_unit() {
        assert_eq!(amount_to_smallest_unit(Decimal::new(1, 3), 6), "1000");
        assert_eq!(amount_to_smallest_unit(Decimal::new(1, 0), 6), "1000000");
    }
}
