use ngx::http::Request;

pub fn log_debug(_r: Option<&Request>, msg: &str) {
    log::debug!("{msg}");
}

pub fn log_info(_r: Option<&Request>, msg: &str) {
    log::info!("{msg}");
}

pub fn log_warn(_r: Option<&Request>, msg: &str) {
    log::warn!("{msg}");
}

pub fn log_error(_r: Option<&Request>, msg: &str) {
    log::error!("{msg}");
}
