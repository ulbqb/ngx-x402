pub mod commands;
pub mod config;
pub mod error;
pub mod handler;
pub mod logging;
pub mod metrics;
#[cfg(not(test))]
pub mod module;
pub mod panic_handler;
pub mod redis;
pub mod request;
pub mod requirements;
pub mod response;
pub mod runtime;
#[cfg(test)]
pub mod module {
    use ngx::ffi::ngx_module_t;

    #[used]
    #[allow(non_upper_case_globals)]
    #[no_mangle]
    pub static mut ngx_http_x402_module: ngx_module_t = ngx_module_t::default();
}

pub use config::{FacilitatorFallback, ParsedX402Config, X402Config};
pub use error::{ConfigError, Result};
pub use handler::{x402_handler_impl, HandlerResult};
pub use metrics::X402Metrics;
pub use module::ngx_http_x402_module;
