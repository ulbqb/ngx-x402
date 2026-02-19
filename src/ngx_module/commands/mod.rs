use ngx::ffi::{
    ngx_command_t, ngx_conf_t, ngx_str_t, ngx_uint_t, NGX_CONF_TAKE1, NGX_HTTP_LOC_CONF,
    NGX_HTTP_LOC_CONF_OFFSET,
};
use ngx::ngx_string;
use std::os::raw::c_void;

use crate::ngx_module::config::X402Config;

macro_rules! ngx_conf_set_str_slot {
    ($handler:ident, $field:ident) => {
        unsafe extern "C" fn $handler(
            cf: *mut ngx_conf_t,
            _cmd: *mut ngx_command_t,
            conf: *mut c_void,
        ) -> *mut u8 {
            let conf = &mut *(conf as *mut X402Config);
            let args = (*(*cf).args).elts as *mut ngx_str_t;
            conf.$field = *args.add(1);
            std::ptr::null_mut()
        }
    };
}

unsafe extern "C" fn ngx_http_x402_set(
    cf: *mut ngx_conf_t,
    _cmd: *mut ngx_command_t,
    conf: *mut c_void,
) -> *mut u8 {
    let conf = unsafe { &mut *(conf as *mut X402Config) };
    let args = unsafe { (*(*cf).args).elts as *mut ngx_str_t };
    let val = unsafe { *args.add(1) };
    let val_str = unsafe { ngx::core::NgxStr::from_ngx_str(val) };
    if let Ok(s) = val_str.to_str() {
        conf.enabled = if s == "on" { 1 } else { 0 };
    }
    std::ptr::null_mut()
}

ngx_conf_set_str_slot!(ngx_http_x402_amount_set, amount_str);
ngx_conf_set_str_slot!(ngx_http_x402_pay_to_set, pay_to_str);
ngx_conf_set_str_slot!(ngx_http_x402_facilitator_url_set, facilitator_url_str);
ngx_conf_set_str_slot!(ngx_http_x402_description_set, description_str);
ngx_conf_set_str_slot!(ngx_http_x402_network_set, network_str);
ngx_conf_set_str_slot!(ngx_http_x402_network_id_set, network_id_str);
ngx_conf_set_str_slot!(ngx_http_x402_resource_set, resource_str);
ngx_conf_set_str_slot!(ngx_http_x402_asset_set, asset_str);
ngx_conf_set_str_slot!(ngx_http_x402_asset_decimals_set, asset_decimals_str);
ngx_conf_set_str_slot!(ngx_http_x402_timeout_set, timeout_str);
ngx_conf_set_str_slot!(ngx_http_x402_fallback_set, facilitator_fallback_str);
ngx_conf_set_str_slot!(ngx_http_x402_ttl_set, ttl_str);
ngx_conf_set_str_slot!(ngx_http_x402_redis_url_set, redis_url_str);
ngx_conf_set_str_slot!(ngx_http_x402_replay_ttl_set, replay_ttl_str);

pub static mut NGX_HTTP_X402_COMMANDS: [ngx_command_t; 16] = [
    ngx_command_t {
        name: ngx_string!("x402"),
        type_: (NGX_HTTP_LOC_CONF | NGX_CONF_TAKE1) as ngx_uint_t,
        set: Some(ngx_http_x402_set),
        conf: NGX_HTTP_LOC_CONF_OFFSET,
        offset: 0,
        post: std::ptr::null_mut(),
    },
    ngx_command_t {
        name: ngx_string!("x402_amount"),
        type_: (NGX_HTTP_LOC_CONF | NGX_CONF_TAKE1) as ngx_uint_t,
        set: Some(ngx_http_x402_amount_set),
        conf: NGX_HTTP_LOC_CONF_OFFSET,
        offset: 0,
        post: std::ptr::null_mut(),
    },
    ngx_command_t {
        name: ngx_string!("x402_pay_to"),
        type_: (NGX_HTTP_LOC_CONF | NGX_CONF_TAKE1) as ngx_uint_t,
        set: Some(ngx_http_x402_pay_to_set),
        conf: NGX_HTTP_LOC_CONF_OFFSET,
        offset: 0,
        post: std::ptr::null_mut(),
    },
    ngx_command_t {
        name: ngx_string!("x402_facilitator_url"),
        type_: (NGX_HTTP_LOC_CONF | NGX_CONF_TAKE1) as ngx_uint_t,
        set: Some(ngx_http_x402_facilitator_url_set),
        conf: NGX_HTTP_LOC_CONF_OFFSET,
        offset: 0,
        post: std::ptr::null_mut(),
    },
    ngx_command_t {
        name: ngx_string!("x402_description"),
        type_: (NGX_HTTP_LOC_CONF | NGX_CONF_TAKE1) as ngx_uint_t,
        set: Some(ngx_http_x402_description_set),
        conf: NGX_HTTP_LOC_CONF_OFFSET,
        offset: 0,
        post: std::ptr::null_mut(),
    },
    ngx_command_t {
        name: ngx_string!("x402_network"),
        type_: (NGX_HTTP_LOC_CONF | NGX_CONF_TAKE1) as ngx_uint_t,
        set: Some(ngx_http_x402_network_set),
        conf: NGX_HTTP_LOC_CONF_OFFSET,
        offset: 0,
        post: std::ptr::null_mut(),
    },
    ngx_command_t {
        name: ngx_string!("x402_network_id"),
        type_: (NGX_HTTP_LOC_CONF | NGX_CONF_TAKE1) as ngx_uint_t,
        set: Some(ngx_http_x402_network_id_set),
        conf: NGX_HTTP_LOC_CONF_OFFSET,
        offset: 0,
        post: std::ptr::null_mut(),
    },
    ngx_command_t {
        name: ngx_string!("x402_resource"),
        type_: (NGX_HTTP_LOC_CONF | NGX_CONF_TAKE1) as ngx_uint_t,
        set: Some(ngx_http_x402_resource_set),
        conf: NGX_HTTP_LOC_CONF_OFFSET,
        offset: 0,
        post: std::ptr::null_mut(),
    },
    ngx_command_t {
        name: ngx_string!("x402_asset"),
        type_: (NGX_HTTP_LOC_CONF | NGX_CONF_TAKE1) as ngx_uint_t,
        set: Some(ngx_http_x402_asset_set),
        conf: NGX_HTTP_LOC_CONF_OFFSET,
        offset: 0,
        post: std::ptr::null_mut(),
    },
    ngx_command_t {
        name: ngx_string!("x402_asset_decimals"),
        type_: (NGX_HTTP_LOC_CONF | NGX_CONF_TAKE1) as ngx_uint_t,
        set: Some(ngx_http_x402_asset_decimals_set),
        conf: NGX_HTTP_LOC_CONF_OFFSET,
        offset: 0,
        post: std::ptr::null_mut(),
    },
    ngx_command_t {
        name: ngx_string!("x402_timeout"),
        type_: (NGX_HTTP_LOC_CONF | NGX_CONF_TAKE1) as ngx_uint_t,
        set: Some(ngx_http_x402_timeout_set),
        conf: NGX_HTTP_LOC_CONF_OFFSET,
        offset: 0,
        post: std::ptr::null_mut(),
    },
    ngx_command_t {
        name: ngx_string!("x402_facilitator_fallback"),
        type_: (NGX_HTTP_LOC_CONF | NGX_CONF_TAKE1) as ngx_uint_t,
        set: Some(ngx_http_x402_fallback_set),
        conf: NGX_HTTP_LOC_CONF_OFFSET,
        offset: 0,
        post: std::ptr::null_mut(),
    },
    ngx_command_t {
        name: ngx_string!("x402_ttl"),
        type_: (NGX_HTTP_LOC_CONF | NGX_CONF_TAKE1) as ngx_uint_t,
        set: Some(ngx_http_x402_ttl_set),
        conf: NGX_HTTP_LOC_CONF_OFFSET,
        offset: 0,
        post: std::ptr::null_mut(),
    },
    ngx_command_t {
        name: ngx_string!("x402_redis_url"),
        type_: (NGX_HTTP_LOC_CONF | NGX_CONF_TAKE1) as ngx_uint_t,
        set: Some(ngx_http_x402_redis_url_set),
        conf: NGX_HTTP_LOC_CONF_OFFSET,
        offset: 0,
        post: std::ptr::null_mut(),
    },
    ngx_command_t {
        name: ngx_string!("x402_replay_ttl"),
        type_: (NGX_HTTP_LOC_CONF | NGX_CONF_TAKE1) as ngx_uint_t,
        set: Some(ngx_http_x402_replay_ttl_set),
        conf: NGX_HTTP_LOC_CONF_OFFSET,
        offset: 0,
        post: std::ptr::null_mut(),
    },
    ngx_command_t::empty(),
];
