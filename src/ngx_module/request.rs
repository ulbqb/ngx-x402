use ngx::http::{Method, Request};

pub fn get_header_value(r: &Request, name: &str) -> Option<String> {
    if name.trim().is_empty() {
        return None;
    }
    for (key, value) in r.headers_in_iterator() {
        if let Ok(key_str) = key.to_str() {
            if key_str.eq_ignore_ascii_case(name) {
                return value.to_str().ok().map(|s| s.to_string());
            }
        }
    }
    None
}

pub fn is_browser_request(r: &Request) -> bool {
    let content_type = get_header_value(r, "Content-Type");
    if let Some(ref ct) = content_type {
        if ct.to_lowercase().starts_with("application/json") {
            return false;
        }
    }

    let accept = get_header_value(r, "Accept");
    if let Some(ref accept_header) = accept {
        let lower = accept_header.to_lowercase();
        if lower.contains("text/html") {
            return true;
        }
        if lower.contains("application/json") && !lower.contains("text/html") {
            return false;
        }
    }

    let user_agent = get_header_value(r, "User-Agent");
    user_agent.as_ref().is_some_and(|ua| {
        let lower = ua.to_lowercase();
        let is_browser = lower.contains("mozilla")
            && (lower.contains("chrome")
                || lower.contains("safari")
                || lower.contains("firefox")
                || lower.contains("edge"));
        let is_api = lower.contains("curl")
            || lower.contains("wget")
            || lower.contains("python-requests")
            || lower.contains("go-http-client")
            || lower.contains("postman");
        is_browser && !is_api
    })
}

pub fn is_websocket_request(r: &Request) -> bool {
    let upgrade = get_header_value(r, "Upgrade");
    let connection = get_header_value(r, "Connection");
    let has_upgrade = upgrade
        .as_ref()
        .is_some_and(|u| u.to_lowercase() == "websocket");
    let has_connection = connection
        .as_ref()
        .is_some_and(|c| c.to_lowercase().contains("upgrade"));
    has_upgrade && has_connection
}

pub fn should_skip_method(r: &Request) -> bool {
    let method = r.method();
    method == Method::OPTIONS || method == Method::HEAD || method == Method::TRACE
}

pub fn build_full_url(r: &Request) -> Option<String> {
    let scheme = get_header_value(r, "X-Forwarded-Proto")
        .and_then(|p| {
            let lower = p.to_lowercase();
            if lower == "https" || lower == "http" {
                Some(lower)
            } else {
                None
            }
        })
        .unwrap_or_else(|| "http".to_string());

    let host = get_header_value(r, "Host")?;
    let uri = r.path().to_str().ok()?;
    let uri_normalized = if uri.starts_with('/') {
        uri.to_string()
    } else {
        format!("/{uri}")
    };

    Some(format!("{scheme}://{host}{uri_normalized}"))
}

pub fn infer_mime_type(r: &Request) -> String {
    if let Some(ct) = get_header_value(r, "Content-Type") {
        let mime = ct.split(';').next().unwrap_or("application/json").trim();
        if !mime.is_empty() {
            return mime.to_string();
        }
    }
    if let Some(accept) = get_header_value(r, "Accept") {
        let lower = accept.to_lowercase();
        if lower.contains("application/json") {
            return "application/json".to_string();
        }
        if lower.contains("text/html") {
            return "text/html".to_string();
        }
    }
    "application/json".to_string()
}
