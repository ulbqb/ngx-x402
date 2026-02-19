use crate::ngx_module::config::ParsedX402Config;
use crate::ngx_module::error::{ConfigError, Result};
use crate::ngx_module::request::is_browser_request;
use crate::ngx_module::requirements::{PaymentRequirements, PaymentRequirementsResponse};
use ngx::core::Status;
use ngx::http::{HTTPStatus, Request};

const HTML_PAYWALL_TEMPLATE: &str = r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>402 Payment Required</title>
<style>
body{font-family:system-ui,-apple-system,sans-serif;display:flex;justify-content:center;align-items:center;min-height:100vh;margin:0;background:#f5f5f5;color:#333}
.card{background:#fff;border-radius:12px;padding:2rem;max-width:480px;width:90%;box-shadow:0 2px 12px rgba(0,0,0,.1);text-align:center}
h1{font-size:1.5rem;margin:0 0 .5rem}
.code{font-size:3rem;font-weight:700;color:#6366f1;margin:.5rem 0}
p{color:#666;line-height:1.5}
.info{background:#f8f9fa;border-radius:8px;padding:1rem;margin:1rem 0;font-size:.875rem;text-align:left}
.info dt{font-weight:600;margin-top:.5rem}
.info dd{margin:0 0 .25rem;font-family:monospace;word-break:break-all}
</style>
</head>
<body>
<div class="card">
<div class="code">402</div>
<h1>Payment Required</h1>
<p>{{MESSAGE}}</p>
<div class="info">
<dl>
<dt>Network</dt><dd>{{NETWORK}}</dd>
<dt>Amount</dt><dd>{{AMOUNT}}</dd>
<dt>Pay To</dt><dd>{{PAY_TO}}</dd>
</dl>
</div>
<p style="font-size:.8rem;color:#999">Powered by x402 protocol</p>
</div>
</body>
</html>"#;

fn generate_paywall_html(message: &str, requirements: &[PaymentRequirements]) -> String {
    let req = requirements.first();
    let network = req.map(|r| r.network.as_str()).unwrap_or("unknown");
    let amount = req
        .map(|r| r.max_amount_required.as_str())
        .unwrap_or("0");
    let pay_to = req.map(|r| r.pay_to.as_str()).unwrap_or("unknown");

    HTML_PAYWALL_TEMPLATE
        .replace("{{MESSAGE}}", message)
        .replace("{{NETWORK}}", network)
        .replace("{{AMOUNT}}", amount)
        .replace("{{PAY_TO}}", pay_to)
}

pub fn send_402_response(
    r: &mut Request,
    requirements: &[PaymentRequirements],
    config: &ParsedX402Config,
    error_msg: Option<&str>,
) -> Result<()> {
    r.set_status(HTTPStatus(402));

    let is_browser = is_browser_request(r);
    let error_message = error_msg
        .or(config.description.as_deref())
        .unwrap_or("Payment required");

    let requirements_json = serde_json::to_string(&PaymentRequirementsResponse::new(
        error_message,
        requirements.to_vec(),
    ))
    .unwrap_or_default();
    let requirements_b64 =
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &requirements_json);

    r.add_header_out("X-Payment-Required", &requirements_b64)
        .ok_or_else(|| ConfigError::new("Failed to set X-Payment-Required header"))?;

    if is_browser {
        let html = generate_paywall_html(error_message, requirements);
        r.add_header_out("Content-Type", "text/html; charset=utf-8")
            .ok_or_else(|| ConfigError::new("Failed to set Content-Type header"))?;
        send_response_body(r, html.as_bytes())?;
    } else {
        let response = PaymentRequirementsResponse::new(error_message, requirements.to_vec());
        let json = serde_json::to_string(&response)
            .map_err(|_| ConfigError::new("Failed to serialize response"))?;
        r.add_header_out("Content-Type", "application/json; charset=utf-8")
            .ok_or_else(|| ConfigError::new("Failed to set Content-Type header"))?;
        send_response_body(r, json.as_bytes())?;
    }

    Ok(())
}

pub fn send_response_body(r: &mut Request, body: &[u8]) -> Result<()> {
    use ngx::ffi::{ngx_alloc_chain_link, ngx_create_temp_buf};

    let pool = r.pool();
    let body_len = body.len();

    if body_len == 0 {
        return Err(ConfigError::new("Cannot send empty response body"));
    }

    let buf = unsafe { ngx_create_temp_buf(pool.as_ptr(), body_len) };
    if buf.is_null() {
        return Err(ConfigError::new("Failed to allocate buffer"));
    }

    unsafe {
        let buf_ref = &mut *buf;
        if buf_ref.pos.is_null() {
            return Err(ConfigError::new("Buffer pos is null"));
        }
        let buf_slice = core::slice::from_raw_parts_mut(buf_ref.pos, body_len);
        buf_slice.copy_from_slice(body);
        buf_ref.last = buf_ref.pos.add(body_len);
        buf_ref.set_last_buf(1);
        buf_ref.set_last_in_chain(1);
    }

    let chain = unsafe { ngx_alloc_chain_link(pool.as_ptr()) };
    if chain.is_null() {
        return Err(ConfigError::new("Failed to allocate chain link"));
    }

    unsafe {
        (*chain).buf = buf;
        (*chain).next = core::ptr::null_mut();
    }

    r.set_content_length_n(body_len);

    let status = r.send_header();
    if status != Status::NGX_OK {
        return Err(ConfigError::new(format!(
            "Failed to send header: {status:?}"
        )));
    }

    let chain_ref = unsafe { &mut *chain };
    let status = r.output_filter(chain_ref);
    if status != Status::NGX_OK {
        return Err(ConfigError::new(format!(
            "Failed to send body: {status:?}"
        )));
    }

    Ok(())
}
