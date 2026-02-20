use std::process::Command;

fn main() {
    if let Ok(src_dir) = std::env::var("NGINX_SOURCE_DIR") {
        println!("cargo:rustc-env=NGINX_SOURCE_DIR={src_dir}");
    }

    let nginx_bin = std::env::var("NGINX_BINARY_PATH").unwrap_or_else(|_| "nginx".to_string());
    if let Ok(output) = Command::new(&nginx_bin).arg("-v").output() {
        let version_str = String::from_utf8_lossy(&output.stderr);
        if let Some(version) = version_str.strip_prefix("nginx version: nginx/") {
            let version = version.trim();
            println!("cargo:warning=Detected nginx version: {version}");
        }
    } else {
        println!("cargo:warning=nginx binary not found, using default build settings");
    }

    // Test stubs define ngx_http_core_module etc. - ONLY for unit/integration tests.
    // When building the cdylib for nginx load_module, these stubs must NOT be linked,
    // otherwise our fake ngx_http_core_module overrides nginx's real symbol and breaks loading.
    if std::env::var("CARGO_FEATURE_INTEGRATION_TEST").is_ok() {
        cc::Build::new()
            .file("test_stubs.c")
            .warnings(false)
            .compile("nginx_test_stubs");
    }
}
