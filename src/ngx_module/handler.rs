use crate::ngx_module::config::{FacilitatorFallback, ParsedX402Config};
use crate::ngx_module::error::{user_errors, ConfigError, Result};
use crate::ngx_module::logging::{log_debug, log_error, log_info, log_warn};
use crate::ngx_module::metrics::X402Metrics;
use crate::ngx_module::redis;
use crate::ngx_module::request::{build_full_url, get_header_value, infer_mime_type};
use crate::ngx_module::requirements::create_requirements;
use crate::ngx_module::response::{send_402_response, send_response_body};
use crate::ngx_module::runtime::{get_runtime, settle_payment, verify_payment};
use ngx::http::{HTTPStatus, Request};
use rust_decimal::prelude::ToPrimitive;
use std::time::Instant;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandlerResult {
    PaymentValid,
    ResponseSent,
    Error,
}

pub fn x402_handler_impl(r: &mut Request, config: &ParsedX402Config) -> Result<HandlerResult> {
    let metrics = X402Metrics::get();
    metrics.record_request();

    if !config.enabled {
        return Ok(HandlerResult::PaymentValid);
    }

    // Initialize Redis if configured and not yet initialized
    if let Some(ref redis_url) = config.redis_url {
        if !redis::is_redis_configured() {
            redis::init_redis(redis_url).ok();
        }
    }

    // Resolve effective amount (Redis override or config default)
    let mut effective_config_amount = config.amount;
    let request_path = r.path().to_str().unwrap_or("/").to_string();
    if redis::is_redis_configured() {
        if let Some(price_str) = redis::get_dynamic_price(&request_path) {
            if let Ok(price) = crate::config::validation::parse_amount(&price_str) {
                log_debug(
                    Some(r),
                    &format!("Redis dynamic price for {request_path}: {price}"),
                );
                effective_config_amount = Some(price);
            }
        }
    }

    let working_config = ParsedX402Config {
        amount: effective_config_amount,
        enabled: config.enabled,
        pay_to: config.pay_to.clone(),
        facilitator_url: config.facilitator_url.clone(),
        description: config.description.clone(),
        network: config.network.clone(),
        network_id: config.network_id,
        resource: config.resource.clone(),
        asset: config.asset.clone(),
        asset_decimals: config.asset_decimals,
        timeout: config.timeout,
        facilitator_fallback: config.facilitator_fallback,
        ttl: config.ttl,
        redis_url: config.redis_url.clone(),
        replay_ttl: config.replay_ttl,
    };

    let full_url = build_full_url(r);
    let resource = if let Some(ref configured) = working_config.resource {
        configured.as_str()
    } else if let Some(ref url) = full_url {
        url.as_str()
    } else {
        r.path().to_str().unwrap_or("/")
    };

    let mime_type = infer_mime_type(r);

    log_debug(
        Some(r),
        &format!("x402 handler processing: resource={resource}, mime={mime_type}"),
    );

    let requirements =
        create_requirements(&working_config, resource, Some(&mime_type)).map_err(|e| {
            log_error(Some(r), &format!("Failed to create requirements: {e}"));
            e
        })?;
    let requirements_slice = std::slice::from_ref(&requirements);

    if let Some(amount_f64) = working_config.amount.and_then(|a| a.to_f64()) {
        metrics.record_payment_amount(amount_f64);
    }

    // Check for PAYMENT-SIGNATURE header (x402 v2)
    let payment_header = get_header_value(r, "Payment-Signature");

    if let Some(payment_b64) = payment_header {
        log_debug(Some(r), "Payment header found, verifying...");
        metrics.record_verification_attempt();

        if payment_b64.len() > crate::ngx_module::runtime::MAX_PAYMENT_HEADER_SIZE {
            log_warn(Some(r), "Payment header too large");
            metrics.record_verification_failed();
            metrics.record_402_response();
            send_402_response(
                r,
                requirements_slice,
                &working_config,
                Some(user_errors::INVALID_PAYMENT),
            )?;
            return Ok(HandlerResult::ResponseSent);
        }

        // Replay prevention
        if redis::is_redis_configured() && redis::is_payment_used(&payment_b64) {
            log_warn(Some(r), "Payment replay detected");
            metrics.record_verification_failed();
            metrics.record_402_response();
            send_402_response(
                r,
                requirements_slice,
                &working_config,
                Some(user_errors::REPLAY_DETECTED),
            )?;
            return Ok(HandlerResult::ResponseSent);
        }

        let facilitator_url = working_config
            .facilitator_url
            .as_deref()
            .ok_or_else(|| {
                log_error(Some(r), "Facilitator URL not configured");
                ConfigError::new("Facilitator URL not configured")
            })?;

        let requirements_json = serde_json::to_value(&requirements)
            .map_err(|e| ConfigError::new(format!("Failed to serialize requirements: {e}")))?;

        let timeout = working_config.timeout;
        let runtime = get_runtime()?;
        let verification_start = Instant::now();
        let verification_result = runtime.block_on(async {
            verify_payment(&payment_b64, &requirements_json, facilitator_url, timeout).await
        });
        let duration = verification_start.elapsed().as_secs_f64();
        metrics.record_verification_duration(duration);

        let response = match verification_result {
            Ok(resp) => {
                log_debug(
                    Some(r),
                    &format!(
                        "Verify result: is_valid={}, duration={duration:.3}s",
                        resp.is_valid
                    ),
                );
                resp
            }
            Err(e) => {
                log_error(Some(r), &format!("Facilitator error: {e}"));
                metrics.record_facilitator_error();
                match working_config.facilitator_fallback {
                    FacilitatorFallback::Error => {
                        r.set_status(HTTPStatus(500));
                        r.add_header_out("Content-Type", "text/plain; charset=utf-8")
                            .ok_or_else(|| ConfigError::new("Failed to set header"))?;
                        send_response_body(r, b"Internal server error")?;
                        return Ok(HandlerResult::ResponseSent);
                    }
                    FacilitatorFallback::Pass => {
                        log_info(Some(r), "Facilitator error, passing through");
                        return Ok(HandlerResult::PaymentValid);
                    }
                }
            }
        };

        if response.is_valid {
            log_info(Some(r), "Payment verified successfully");
            metrics.record_verification_success();

            // Settle payment on-chain (execute the actual USDC transfer)
            let settle_result = runtime.block_on(async {
                settle_payment(
                    &payment_b64,
                    &requirements_json,
                    facilitator_url,
                    timeout,
                )
                .await
            });

            match settle_result {
                Ok(settle) => {
                    if !settle.success {
                        let err_info = [
                            settle.error_reason.as_deref().unwrap_or(""),
                            settle.error_message.as_deref().unwrap_or(""),
                        ]
                        .iter()
                        .filter(|s| !s.is_empty())
                        .cloned()
                        .collect::<Vec<_>>()
                        .join("; ");
                        log_error(
                            Some(r),
                            &format!(
                                "Payment settle failed: success=false txHash={:?} {}",
                                settle.tx_hash.as_deref().unwrap_or("none"),
                                if err_info.is_empty() {
                                    "".to_string()
                                } else {
                                    format!("errorReason/Message: {err_info}")
                                }
                            ),
                        );
                        metrics.record_verification_failed();
                        let err_msg = if err_info.is_empty() {
                            user_errors::PAYMENT_VERIFICATION_FAILED.to_string()
                        } else {
                            format!(
                                "{} (Facilitator: {err_info})",
                                user_errors::PAYMENT_VERIFICATION_FAILED
                            )
                        };
                        send_402_response(
                            r,
                            requirements_slice,
                            &working_config,
                            Some(&err_msg),
                        )?;
                        return Ok(HandlerResult::ResponseSent);
                    }
                    log_info(
                        Some(r),
                        &format!(
                            "Payment settled on-chain, txHash={:?}",
                            settle.tx_hash.as_deref().unwrap_or("none")
                        ),
                    );
                }
                Err(e) => {
                    log_error(Some(r), &format!("Payment settlement failed: {e}"));
                    metrics.record_verification_failed();
                    let err_msg = format!(
                        "{} (Facilitator error: {e})",
                        user_errors::PAYMENT_VERIFICATION_FAILED
                    );
                    send_402_response(
                        r,
                        requirements_slice,
                        &working_config,
                        Some(&err_msg),
                    )?;
                    return Ok(HandlerResult::ResponseSent);
                }
            }

            // Store as used for replay prevention
            if redis::is_redis_configured() {
                let ttl = working_config.replay_ttl.unwrap_or(86400);
                redis::store_payment_as_used(&payment_b64, ttl).ok();
            }

            Ok(HandlerResult::PaymentValid)
        } else {
            log_warn(Some(r), "Payment verification failed (is_valid=false)");
            metrics.record_verification_failed();
            metrics.record_402_response();
            send_402_response(
                r,
                requirements_slice,
                &working_config,
                Some(user_errors::PAYMENT_VERIFICATION_FAILED),
            )?;
            Ok(HandlerResult::ResponseSent)
        }
    } else {
        log_debug(Some(r), "No payment header found, sending 402");
        metrics.record_402_response();
        send_402_response(r, requirements_slice, &working_config, None)?;
        Ok(HandlerResult::ResponseSent)
    }
}
