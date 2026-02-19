use crate::ngx_module::error::{ConfigError, Result};
use crate::ngx_module::logging::{log_debug, log_error, log_warn};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::Duration;

pub static RUNTIME: OnceLock<tokio::runtime::Runtime> = OnceLock::new();

pub static FACILITATOR_CLIENTS: OnceLock<Mutex<HashMap<String, Arc<HttpFacilitatorClient>>>> =
    OnceLock::new();

pub const DEFAULT_FACILITATOR_TIMEOUT: Duration = Duration::from_secs(10);
pub const MAX_PAYMENT_HEADER_SIZE: usize = 64 * 1024;

pub fn get_runtime() -> Result<&'static tokio::runtime::Runtime> {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Runtime::new()
            .unwrap_or_else(|e| panic!("Failed to create tokio runtime: {e}"))
    });
    RUNTIME
        .get()
        .ok_or_else(|| ConfigError::new("Runtime not initialized"))
}

pub struct HttpFacilitatorClient {
    http_client: reqwest::Client,
    base_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VerifyRequestBody {
    #[serde(rename = "paymentPayload")]
    pub payment_payload: serde_json::Value,
    #[serde(rename = "paymentRequirements")]
    pub payment_requirements: serde_json::Value,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct VerifyResponseBody {
    #[serde(rename = "isValid")]
    pub is_valid: bool,
    #[serde(rename = "invalidReason")]
    pub invalid_reason: Option<String>,
    #[serde(rename = "payer")]
    pub payer: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct SettleResponseBody {
    pub success: bool,
    #[serde(rename = "txHash")]
    pub tx_hash: Option<String>,
}

impl HttpFacilitatorClient {
    pub fn new(base_url: &str) -> Result<Self> {
        let http_client = reqwest::Client::builder()
            .build()
            .map_err(|e| ConfigError::new(format!("Failed to create HTTP client: {e}")))?;
        Ok(Self {
            http_client,
            base_url: base_url.trim_end_matches('/').to_string(),
        })
    }

    pub async fn verify(
        &self,
        body: &VerifyRequestBody,
        timeout: Duration,
    ) -> Result<VerifyResponseBody> {
        let url = format!("{}/verify", self.base_url);
        let resp = self
            .http_client
            .post(&url)
            .json(body)
            .timeout(timeout)
            .send()
            .await
            .map_err(|e| ConfigError::new(format!("Facilitator verify request failed: {e}")))?;

        if !resp.status().is_success() {
            return Err(ConfigError::new(format!(
                "Facilitator returned status {}",
                resp.status()
            )));
        }

        resp.json::<VerifyResponseBody>()
            .await
            .map_err(|e| ConfigError::new(format!("Failed to parse verify response: {e}")))
    }

    pub async fn settle(
        &self,
        body: &VerifyRequestBody,
        timeout: Duration,
    ) -> Result<SettleResponseBody> {
        let url = format!("{}/settle", self.base_url);
        let resp = self
            .http_client
            .post(&url)
            .json(body)
            .timeout(timeout)
            .send()
            .await
            .map_err(|e| ConfigError::new(format!("Facilitator settle request failed: {e}")))?;

        if !resp.status().is_success() {
            return Err(ConfigError::new(format!(
                "Facilitator settle returned status {}",
                resp.status()
            )));
        }

        resp.json::<SettleResponseBody>()
            .await
            .map_err(|e| ConfigError::new(format!("Failed to parse settle response: {e}")))
    }
}

pub fn get_facilitator_client(url: &str) -> Result<Arc<HttpFacilitatorClient>> {
    let clients = FACILITATOR_CLIENTS.get_or_init(|| Mutex::new(HashMap::new()));

    {
        let guard = clients
            .lock()
            .map_err(|_| ConfigError::new("Lock poisoned"))?;
        if let Some(client) = guard.get(url) {
            return Ok(Arc::clone(client));
        }
    }

    let client = HttpFacilitatorClient::new(url)?;
    let client_arc = Arc::new(client);

    {
        let mut guard = clients
            .lock()
            .map_err(|_| ConfigError::new("Lock poisoned"))?;
        guard.insert(url.to_string(), Arc::clone(&client_arc));
    }

    Ok(client_arc)
}

pub async fn verify_payment(
    payment_b64: &str,
    requirements_json: &serde_json::Value,
    facilitator_url: &str,
    timeout_duration: Option<Duration>,
) -> Result<VerifyResponseBody> {
    use crate::ngx_module::error::user_errors;

    if payment_b64.is_empty() {
        return Err(ConfigError::new(user_errors::INVALID_PAYMENT));
    }
    if facilitator_url.is_empty() {
        return Err(ConfigError::new(user_errors::CONFIGURATION_ERROR));
    }

    let payment_payload: serde_json::Value = serde_json::from_slice(
        &base64::Engine::decode(&base64::engine::general_purpose::STANDARD, payment_b64)
            .map_err(|e| {
                log_error(None, &format!("Failed to decode payment payload: {e}"));
                ConfigError::new(user_errors::INVALID_PAYMENT)
            })?,
    )
    .map_err(|e| {
        log_error(None, &format!("Failed to parse payment JSON: {e}"));
        ConfigError::new(user_errors::INVALID_PAYMENT)
    })?;

    let body = VerifyRequestBody {
        payment_payload,
        payment_requirements: requirements_json.clone(),
    };

    let client = get_facilitator_client(facilitator_url)?;
    let timeout = timeout_duration.unwrap_or(DEFAULT_FACILITATOR_TIMEOUT);

    match tokio::time::timeout(timeout, client.verify(&body, timeout)).await {
        Ok(Ok(response)) => {
            log_debug(
                None,
                &format!(
                    "Facilitator verify response: is_valid={}, reason={:?}",
                    response.is_valid,
                    response.invalid_reason.as_deref().unwrap_or("none")
                ),
            );
            Ok(response)
        }
        Ok(Err(e)) => {
            log_error(None, &format!("Payment verification failed: {e}"));
            Err(ConfigError::new(user_errors::PAYMENT_VERIFICATION_FAILED))
        }
        Err(_) => {
            log_warn(
                None,
                &format!("Payment verification timeout after {timeout:?}"),
            );
            Err(ConfigError::new(user_errors::TIMEOUT))
        }
    }
}
