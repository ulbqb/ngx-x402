use rust_decimal::Decimal;
use std::str::FromStr;
use x402_types::chain::ChainId;

pub fn validate_amount(amount: Decimal) -> Result<(), String> {
    if amount < Decimal::ZERO {
        return Err("Amount cannot be negative".to_string());
    }
    if amount.scale() > 18 {
        return Err(format!(
            "Amount has {} decimal places, maximum is 18",
            amount.scale()
        ));
    }
    Ok(())
}

pub fn validate_ethereum_address(address: &str) -> Result<(), String> {
    let addr = address.trim();
    if !addr.starts_with("0x") && !addr.starts_with("0X") {
        return Err("Ethereum address must start with 0x".to_string());
    }
    if addr.len() != 42 {
        return Err(format!(
            "Ethereum address must be 42 characters, got {}",
            addr.len()
        ));
    }
    if !addr[2..].chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("Ethereum address contains invalid hex characters".to_string());
    }
    Ok(())
}

pub fn validate_network(network: &str) -> Result<(), String> {
    let net = network.trim();
    if net.is_empty() {
        return Err("Network cannot be empty".to_string());
    }

    // Accept CAIP-2 format (e.g., "eip155:8453")
    if net.contains(':') {
        let chain =
            ChainId::from_str(net).map_err(|_| format!("Invalid CAIP-2 network format: {net}"))?;
        if chain.namespace().is_empty() || chain.reference().is_empty() {
            return Err(format!("Invalid CAIP-2 network format: {net}"));
        }
        return Ok(());
    }

    // Or known friendly names supported by x402-types (e.g., "base-sepolia")
    if ChainId::from_network_name(net).is_none() {
        return Err(format!("Unsupported network name: {net}"));
    }

    Ok(())
}

pub fn validate_url(url: &str) -> Result<(), String> {
    let u = url.trim();
    if u.is_empty() {
        return Err("URL cannot be empty".to_string());
    }
    if !u.starts_with("http://") && !u.starts_with("https://") {
        return Err("URL must start with http:// or https://".to_string());
    }
    Ok(())
}

pub fn validate_resource_path(path: &str) -> Result<String, String> {
    let p = path.trim();
    if p.is_empty() {
        return Err("Resource path cannot be empty".to_string());
    }
    if p.contains("..") {
        return Err("Resource path cannot contain '..'".to_string());
    }
    Ok(p.to_string())
}

pub fn chain_id_to_network(chain_id: u64) -> Result<&'static str, String> {
    let chain = ChainId::new("eip155", chain_id.to_string());
    chain
        .as_network_name()
        .ok_or_else(|| format!("Unsupported chain ID: {chain_id}"))
}

pub fn parse_amount(s: &str) -> Result<Decimal, String> {
    let s = s.trim();
    // Support dollar-prefixed amounts like "$0.001"
    let s = s.strip_prefix('$').unwrap_or(s);
    Decimal::from_str(s).map_err(|e| format!("Invalid amount: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_amount() {
        assert!(validate_amount(Decimal::from_str("0.001").unwrap()).is_ok());
        assert!(validate_amount(Decimal::ZERO).is_ok());
        assert!(validate_amount(Decimal::from_str("100").unwrap()).is_ok());
        assert!(validate_amount(Decimal::from_str("-0.001").unwrap()).is_err());
        assert!(validate_amount(Decimal::from_str("-1").unwrap()).is_err());
    }

    #[test]
    fn test_validate_amount_scale_too_high() {
        let d = Decimal::from_str("0.0000000000000000001").unwrap();
        assert!(validate_amount(d).is_err());
    }

    #[test]
    fn test_validate_ethereum_address() {
        assert!(validate_ethereum_address("0x1234567890abcdef1234567890abcdef12345678").is_ok());
        assert!(validate_ethereum_address("0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913").is_ok());
        assert!(validate_ethereum_address("0x0000000000000000000000000000000000000000").is_ok());
        assert!(validate_ethereum_address("not_an_address").is_err());
        assert!(validate_ethereum_address("1234567890abcdef1234567890abcdef12345678").is_err());
        assert!(validate_ethereum_address("0x1234").is_err());
        assert!(validate_ethereum_address("0xGGGG567890abcdef1234567890abcdef12345678").is_err());
    }

    #[test]
    fn test_validate_network() {
        assert!(validate_network("base-sepolia").is_ok());
        assert!(validate_network("base").is_ok());
        assert!(validate_network("eip155:8453").is_ok());
        assert!(validate_network("eip155:84532").is_ok());
        assert!(validate_network("solana:5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp").is_ok());
        assert!(validate_network("").is_err());
        assert!(validate_network(":").is_err());
    }

    #[test]
    fn test_validate_url() {
        assert!(validate_url("https://example.com/facilitator").is_ok());
        assert!(validate_url("https://x402.org/facilitator").is_ok());
        assert!(validate_url("http://localhost:8080").is_ok());
        assert!(validate_url("http://localhost:8080/verify").is_ok());
        assert!(validate_url("ftp://invalid").is_err());
        assert!(validate_url("").is_err());
    }

    #[test]
    fn test_validate_resource_path() {
        assert!(validate_resource_path("/api/weather").is_ok());
        assert!(validate_resource_path("https://example.com/api").is_ok());
        assert!(validate_resource_path("https://example.com/api/weather").is_ok());
        assert!(validate_resource_path("").is_err());
        assert!(validate_resource_path("/api/../secret").is_err());
        assert!(validate_resource_path("/api/../etc/passwd").is_err());
    }

    #[test]
    fn test_chain_id_to_network() {
        assert_eq!(chain_id_to_network(8453).unwrap(), "base");
        assert_eq!(chain_id_to_network(84532).unwrap(), "base-sepolia");
        assert_eq!(chain_id_to_network(137).unwrap(), "polygon");
        assert!(chain_id_to_network(999999).is_err());
    }

    #[test]
    fn test_parse_amount() {
        assert_eq!(
            parse_amount("0.001").unwrap(),
            Decimal::from_str("0.001").unwrap()
        );
        assert_eq!(
            parse_amount("$0.001").unwrap(),
            Decimal::from_str("0.001").unwrap()
        );
        assert_eq!(parse_amount("$1").unwrap(), Decimal::from_str("1").unwrap());
        assert!(parse_amount("abc").is_err());
        assert!(parse_amount("").is_err());
    }
}
