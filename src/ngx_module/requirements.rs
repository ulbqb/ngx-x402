use crate::config::validation::chain_id_to_network;
use crate::ngx_module::config::ParsedX402Config;
use crate::ngx_module::error::{ConfigError, Result};
use rust_decimal::Decimal;
use std::str::FromStr;
use x402_types::chain::ChainId;
use x402_types::proto::v2::{PaymentRequired, ResourceInfo, X402Version2};

pub type PaymentRequirements = x402_types::proto::v2::PaymentRequirements;
pub type PaymentRequiredResponse = PaymentRequired<PaymentRequirements>;

fn amount_to_smallest_unit(amount: Decimal, decimals: u8) -> String {
    let multiplier = Decimal::from(10u64.pow(decimals as u32));
    (amount * multiplier).normalize().to_string()
}

fn resolve_network(config: &ParsedX402Config) -> Result<ChainId> {
    if let Some(chain_id) = config.network_id {
        chain_id_to_network(chain_id).map_err(ConfigError::new)?;
        Ok(ChainId::new("eip155", chain_id.to_string()))
    } else if let Some(ref net) = config.network {
        if net.contains(':') {
            ChainId::from_str(net)
                .map_err(|_| ConfigError::new(format!("Invalid CAIP-2 network format: {net}")))
        } else {
            ChainId::from_network_name(net)
                .ok_or_else(|| ConfigError::new(format!("Unsupported network name: {net}")))
        }
    } else {
        Ok(ChainId::new("eip155", "8453"))
    }
}

fn default_usdc_address(network: &ChainId) -> Option<&'static str> {
    match (network.namespace.as_str(), network.reference.as_str()) {
        ("eip155", "8453") => Some("0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"),
        ("eip155", "84532") => Some("0x036CbD53842c5426634e7929541eC2318f3dCF7e"),
        ("eip155", "137") => Some("0x3c499c542cEF5E3811e1192ce70d8cC03d5c3359"),
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
    let amount_str = amount_to_smallest_unit(amount, decimals);
    let asset_address = if let Some(ref custom) = config.asset {
        custom.clone()
    } else {
        default_usdc_address(&network)
            .map(|s| s.to_string())
            .unwrap_or_default()
    };
    let resource = crate::config::validation::validate_resource_path(resource)
        .map_err(|e| ConfigError::new(e))?;
    if resource.is_empty() {
        return Err(ConfigError::new("Resource path cannot be empty"));
    }
    let max_timeout_seconds = config.ttl.unwrap_or(60);
    let extra = eip712_extra_for_asset(&asset_address);
    Ok(PaymentRequirements {
        scheme: "exact".to_string(),
        network,
        amount: amount_str,
        pay_to: pay_to.to_lowercase(),
        max_timeout_seconds: max_timeout_seconds as u64,
        asset: asset_address,
        extra,
    })
}

pub fn create_payment_required_response(
    error: &str,
    accepts: Vec<PaymentRequirements>,
    resource_url: &str,
    description: &str,
    mime_type: &str,
) -> PaymentRequiredResponse {
    PaymentRequiredResponse {
        x402_version: X402Version2,
        error: Some(error.to_string()),
        resource: ResourceInfo {
            description: description.to_string(),
            mime_type: mime_type.to_string(),
            url: resource_url.to_string(),
        },
        accepts,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ngx_module::config::FacilitatorFallback;
    use std::str::FromStr;
    use std::time::Duration;

    fn test_config(
        amount: Option<Decimal>,
        pay_to: Option<String>,
        network: Option<String>,
        network_id: Option<u64>,
        asset: Option<String>,
        asset_decimals: Option<u8>,
        ttl: Option<u32>,
        description: Option<String>,
    ) -> ParsedX402Config {
        ParsedX402Config {
            enabled: true,
            amount,
            pay_to,
            facilitator_url: Some("https://example.com/facilitator".to_string()),
            description,
            network,
            network_id,
            resource: None,
            asset,
            asset_decimals,
            timeout: Some(Duration::from_secs(10)),
            facilitator_fallback: FacilitatorFallback::Error,
            ttl,
            redis_url: None,
            replay_ttl: None,
        }
    }

    #[test]
    fn test_amount_to_smallest_unit() {
        assert_eq!(amount_to_smallest_unit(Decimal::new(1, 3), 6), "1000");
        assert_eq!(amount_to_smallest_unit(Decimal::new(1, 0), 6), "1000000");
    }

    #[test]
    fn test_create_requirements_success() {
        let config = test_config(
            Some(Decimal::from_str("0.001").unwrap()),
            Some("0x1234567890abcdef1234567890abcdef12345678".to_string()),
            None,
            None,
            None,
            None,
            None,
            None,
        );
        let req = create_requirements(&config, "/api/weather").unwrap();
        assert_eq!(req.scheme, "exact");
        assert_eq!(req.network.to_string(), "eip155:8453");
        assert_eq!(req.amount, "1000");
        assert_eq!(req.max_timeout_seconds, 60);
        assert_eq!(req.asset, "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913");
    }

    #[test]
    fn test_create_requirements_network_from_network_id() {
        let config = test_config(
            Some(Decimal::from_str("0.001").unwrap()),
            Some("0x1234567890abcdef1234567890abcdef12345678".to_string()),
            None,
            Some(8453),
            None,
            None,
            None,
            None,
        );
        let req = create_requirements(&config, "/api").unwrap();
        assert_eq!(req.network.to_string(), "eip155:8453");
    }

    #[test]
    fn test_create_requirements_network_from_name() {
        let config = test_config(
            Some(Decimal::from_str("0.001").unwrap()),
            Some("0x1234567890abcdef1234567890abcdef12345678".to_string()),
            Some("base".to_string()),
            None,
            None,
            None,
            None,
            None,
        );
        let req = create_requirements(&config, "/api").unwrap();
        assert_eq!(req.network.to_string(), "eip155:8453");
    }

    #[test]
    fn test_create_requirements_default_usdc() {
        let config = test_config(
            Some(Decimal::from_str("0.001").unwrap()),
            Some("0x1234567890abcdef1234567890abcdef12345678".to_string()),
            None,
            None,
            None,
            None,
            None,
            None,
        );
        let req = create_requirements(&config, "/api").unwrap();
        assert_eq!(req.asset, "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913");
        assert!(req.extra.is_some());
    }

    #[test]
    fn test_create_requirements_custom_asset() {
        let config = test_config(
            Some(Decimal::from_str("0.001").unwrap()),
            Some("0x1234567890abcdef1234567890abcdef12345678".to_string()),
            Some("base-sepolia".to_string()),
            None,
            Some("0x036CbD53842c5426634e7929541eC2318f3dCF7e".to_string()),
            None,
            None,
            None,
        );
        let req = create_requirements(&config, "/api").unwrap();
        assert_eq!(req.asset, "0x036CbD53842c5426634e7929541eC2318f3dCF7e");
        let extra = req.extra.as_ref().unwrap();
        assert_eq!(extra.get("name").and_then(|v| v.as_str()), Some("USDC"));
    }

    #[test]
    fn test_create_requirements_missing_amount() {
        let config = test_config(
            None,
            Some("0x1234567890abcdef1234567890abcdef12345678".to_string()),
            None,
            None,
            None,
            None,
            None,
            None,
        );
        assert!(create_requirements(&config, "/api").is_err());
    }

    #[test]
    fn test_create_requirements_missing_pay_to() {
        let config = test_config(
            Some(Decimal::from_str("0.001").unwrap()),
            None,
            None,
            None,
            None,
            None,
            None,
            None,
        );
        assert!(create_requirements(&config, "/api").is_err());
    }

    #[test]
    fn test_create_requirements_negative_amount() {
        let config = test_config(
            Some(Decimal::from_str("-0.001").unwrap()),
            Some("0x1234567890abcdef1234567890abcdef12345678".to_string()),
            None,
            None,
            None,
            None,
            None,
            None,
        );
        assert!(create_requirements(&config, "/api").is_err());
    }

    #[test]
    fn test_payment_requirements_json_v2_format() {
        let config = test_config(
            Some(Decimal::from_str("0.001").unwrap()),
            Some("0x1234567890abcdef1234567890abcdef12345678".to_string()),
            None,
            None,
            None,
            None,
            None,
            None,
        );
        let req = create_requirements(&config, "/api/weather").unwrap();
        let json = serde_json::to_value(&req).unwrap();
        assert!(json.get("amount").is_some(), "amount must be present (v2)");
        assert!(
            json.get("maxAmountRequired").is_none(),
            "maxAmountRequired must not be present (v1 removed)"
        );
    }

    #[test]
    fn test_payment_requirements_response_x402_version_2() {
        let req = create_requirements(
            &test_config(
                Some(Decimal::from_str("0.001").unwrap()),
                Some("0x1234567890abcdef1234567890abcdef12345678".to_string()),
                None,
                None,
                None,
                None,
                None,
                None,
            ),
            "/api",
        )
        .unwrap();
        let resp = create_payment_required_response(
            "Payment required",
            vec![req],
            "/api",
            "desc",
            "application/json",
        );
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(
            json.get("x402Version").and_then(|v| v.as_u64()),
            Some(2),
            "x402Version must be 2"
        );
    }

    #[test]
    fn test_402_response_json_v2_structure() {
        let config = test_config(
            Some(Decimal::from_str("0.001").unwrap()),
            Some("0x1234567890abcdef1234567890abcdef12345678".to_string()),
            None,
            None,
            None,
            None,
            None,
            None,
        );
        let req = create_requirements(&config, "/api").unwrap();
        let resp =
            create_payment_required_response("Pay", vec![req], "/api", "desc", "application/json");
        let json = serde_json::to_value(&resp).unwrap();
        assert_eq!(json["x402Version"], 2);
        let accept = &json["accepts"][0];
        assert!(accept.get("amount").is_some());
        assert!(accept["network"].as_str().unwrap().starts_with("eip155:"));
    }
}
