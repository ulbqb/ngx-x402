use ngx::ffi::{
    ngx_array_push, ngx_conf_t, ngx_http_core_main_conf_t, ngx_http_handler_pt, ngx_http_module_t,
    ngx_http_phases_NGX_HTTP_ACCESS_PHASE, ngx_int_t, ngx_module_t, NGX_HTTP_MODULE,
};
use ngx::http::{
    HttpModule, HttpModuleLocationConf, HttpModuleMainConf, Merge, MergeConfigError,
    NgxHttpCoreModule, Request,
};
use std::ffi::c_char;
use std::os::raw::c_void;
use std::ptr;

use crate::ngx_module::commands::NGX_HTTP_X402_COMMANDS;
use crate::ngx_module::config::X402Config;

pub struct X402Module;

impl HttpModule for X402Module {
    fn module() -> &'static ngx_module_t {
        unsafe { &*ptr::addr_of!(ngx_http_x402_module) }
    }
}

unsafe impl HttpModuleLocationConf for X402Module {
    type LocationConf = X402Config;
}

impl Merge for X402Config {
    fn merge(&mut self, prev: &X402Config) -> Result<(), MergeConfigError> {
        if prev.enabled != 0 && self.enabled == 0 {
            self.enabled = prev.enabled;
        }
        macro_rules! merge_str {
            ($field:ident) => {
                if self.$field.len == 0 && prev.$field.len > 0 {
                    self.$field = prev.$field;
                }
            };
        }
        merge_str!(amount_str);
        merge_str!(pay_to_str);
        merge_str!(facilitator_url_str);
        merge_str!(description_str);
        merge_str!(network_str);
        merge_str!(network_id_str);
        merge_str!(resource_str);
        merge_str!(asset_str);
        merge_str!(asset_decimals_str);
        merge_str!(timeout_str);
        merge_str!(facilitator_fallback_str);
        merge_str!(ttl_str);
        merge_str!(redis_url_str);
        merge_str!(replay_ttl_str);
        Ok(())
    }
}

pub fn get_loc_conf(r: &Request) -> Option<&'static X402Config> {
    X402Module::location_conf(r)
}

unsafe extern "C" fn postconfiguration(cf: *mut ngx_conf_t) -> ngx_int_t {
    let cmcf = match NgxHttpCoreModule::main_conf_mut(&*cf) {
        Some(c) => c,
        None => return ngx::ffi::NGX_ERROR as ngx_int_t,
    };

    let phase_idx = ngx_http_phases_NGX_HTTP_ACCESS_PHASE as usize;
    let handlers_ptr = ptr::addr_of_mut!(cmcf.phases[phase_idx].handlers);
    let h = ngx_array_push(handlers_ptr) as *mut ngx_http_handler_pt;

    if h.is_null() {
        return ngx::ffi::NGX_ERROR as ngx_int_t;
    }

    *h = Some(x402_phase_handler);
    ngx::ffi::NGX_OK as ngx_int_t
}

unsafe extern "C" fn create_loc_conf(cf: *mut ngx_conf_t) -> *mut c_void {
    let pool = ngx::core::Pool::from_ngx_pool((*cf).pool);
    pool.allocate::<X402Config>(Default::default()) as *mut c_void
}

unsafe extern "C" fn merge_loc_conf(
    _cf: *mut ngx_conf_t,
    prev: *mut c_void,
    conf: *mut c_void,
) -> *mut c_char {
    let prev = &*(prev as *const X402Config);
    let conf = &mut *(conf as *mut X402Config);
    match conf.merge(prev) {
        Ok(_) => ptr::null_mut(),
        Err(_) => ngx::core::NGX_CONF_ERROR as *mut c_char,
    }
}

pub static NGX_HTTP_X402_MODULE_CTX: ngx_http_module_t = ngx_http_module_t {
    preconfiguration: None,
    postconfiguration: Some(postconfiguration),
    create_main_conf: None,
    init_main_conf: None,
    create_srv_conf: None,
    merge_srv_conf: None,
    create_loc_conf: Some(create_loc_conf),
    merge_loc_conf: Some(merge_loc_conf),
};

#[cfg(feature = "export-modules")]
ngx::ngx_modules!(ngx_http_x402_module);

#[used]
#[allow(non_upper_case_globals)]
#[no_mangle]
pub static mut ngx_http_x402_module: ngx_module_t = {
    let mut m = ngx_module_t::default();
    m.ctx = &NGX_HTTP_X402_MODULE_CTX as *const _ as *mut c_void;
    m.commands = unsafe { &NGX_HTTP_X402_COMMANDS[0] as *const _ as *mut _ };
    m.type_ = NGX_HTTP_MODULE as usize;
    m
};

/// ACCESS_PHASE handler - called before proxy_pass content handler.
///
/// # Safety
/// Called by nginx with a valid request pointer.
#[no_mangle]
pub unsafe extern "C" fn x402_phase_handler(
    r: *mut ngx::ffi::ngx_http_request_t,
) -> ngx::ffi::ngx_int_t {
    use crate::ngx_module::panic_handler::catch_panic_or_default;

    if r.is_null() {
        return ngx::ffi::NGX_ERROR as ngx::ffi::ngx_int_t;
    }

    catch_panic_or_default(
        || {
            let req = unsafe { Request::from_ngx_http_request(r) };

            use crate::ngx_module::handler::HandlerResult;
            use crate::ngx_module::request::{is_websocket_request, should_skip_method};

            if should_skip_method(req) {
                return ngx::ffi::NGX_DECLINED as ngx::ffi::ngx_int_t;
            }

            if is_websocket_request(req) {
                return ngx::ffi::NGX_DECLINED as ngx::ffi::ngx_int_t;
            }

            // Skip subrequests
            if !req.is_main() {
                return ngx::ffi::NGX_DECLINED as ngx::ffi::ngx_int_t;
            }

            let conf = match get_loc_conf(req) {
                Some(c) => c,
                None => return ngx::ffi::NGX_DECLINED as ngx::ffi::ngx_int_t,
            };

            if conf.enabled == 0 {
                return ngx::ffi::NGX_DECLINED as ngx::ffi::ngx_int_t;
            }

            let parsed = match conf.parse() {
                Ok(c) => c,
                Err(e) => {
                    log::error!("Failed to parse x402 config: {e}");
                    return ngx::ffi::NGX_ERROR as ngx::ffi::ngx_int_t;
                }
            };

            match crate::ngx_module::handler::x402_handler_impl(req, &parsed) {
                Ok(HandlerResult::PaymentValid) => ngx::ffi::NGX_DECLINED as ngx::ffi::ngx_int_t,
                Ok(HandlerResult::ResponseSent) => ngx::ffi::NGX_OK as ngx::ffi::ngx_int_t,
                Ok(HandlerResult::Error) | Err(_) => {
                    ngx::ffi::NGX_HTTP_INTERNAL_SERVER_ERROR as ngx::ffi::ngx_int_t
                }
            }
        },
        "x402_phase_handler",
        ngx::ffi::NGX_ERROR as ngx::ffi::ngx_int_t,
    )
}
