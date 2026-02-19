use ngx_x402::config::validation;
use rust_decimal::Decimal;
use std::str::FromStr;

#[test]
fn test_validate_amount_positive() {
    assert!(validation::validate_amount(Decimal::from_str("0.001").unwrap()).is_ok());
    assert!(validation::validate_amount(Decimal::ZERO).is_ok());
    assert!(validation::validate_amount(Decimal::from_str("100").unwrap()).is_ok());
}

#[test]
fn test_validate_amount_negative() {
    assert!(validation::validate_amount(Decimal::from_str("-0.001").unwrap()).is_err());
}

#[test]
fn test_validate_ethereum_address_valid() {
    assert!(
        validation::validate_ethereum_address("0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913")
            .is_ok()
    );
    assert!(
        validation::validate_ethereum_address("0x0000000000000000000000000000000000000000")
            .is_ok()
    );
}

#[test]
fn test_validate_ethereum_address_invalid() {
    assert!(validation::validate_ethereum_address("not_an_address").is_err());
    assert!(validation::validate_ethereum_address("0x1234").is_err());
    assert!(
        validation::validate_ethereum_address("0xGGGG567890abcdef1234567890abcdef12345678")
            .is_err()
    );
}

#[test]
fn test_validate_network() {
    assert!(validation::validate_network("base-sepolia").is_ok());
    assert!(validation::validate_network("base").is_ok());
    assert!(validation::validate_network("eip155:8453").is_ok());
    assert!(validation::validate_network("eip155:84532").is_ok());
    assert!(validation::validate_network("solana:5eykt4UsFv8P8NJdTREpY1vzqKqZKvdp").is_ok());
    assert!(validation::validate_network("").is_err());
}

#[test]
fn test_validate_url() {
    assert!(validation::validate_url("https://x402.org/facilitator").is_ok());
    assert!(validation::validate_url("http://localhost:8080/verify").is_ok());
    assert!(validation::validate_url("ftp://invalid").is_err());
    assert!(validation::validate_url("").is_err());
}

#[test]
fn test_validate_resource_path() {
    assert!(validation::validate_resource_path("/api/weather").is_ok());
    assert!(validation::validate_resource_path("https://example.com/api/weather").is_ok());
    assert!(validation::validate_resource_path("").is_err());
    assert!(validation::validate_resource_path("/api/../etc/passwd").is_err());
}

#[test]
fn test_chain_id_to_network() {
    assert_eq!(validation::chain_id_to_network(8453).unwrap(), "base");
    assert_eq!(
        validation::chain_id_to_network(84532).unwrap(),
        "base-sepolia"
    );
    assert_eq!(validation::chain_id_to_network(137).unwrap(), "polygon");
    assert!(validation::chain_id_to_network(999999).is_err());
}

#[test]
fn test_parse_amount() {
    assert_eq!(
        validation::parse_amount("0.001").unwrap(),
        Decimal::from_str("0.001").unwrap()
    );
    assert_eq!(
        validation::parse_amount("$0.001").unwrap(),
        Decimal::from_str("0.001").unwrap()
    );
    assert_eq!(
        validation::parse_amount("$1").unwrap(),
        Decimal::from_str("1").unwrap()
    );
    assert!(validation::parse_amount("abc").is_err());
    assert!(validation::parse_amount("").is_err());
}
