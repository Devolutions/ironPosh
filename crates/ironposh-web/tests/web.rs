//! Test suite for the Web and headless browsers.

#![cfg(target_arch = "wasm32")]

extern crate wasm_bindgen_test;
use ironposh_web::{
    GatewayTransport, SecurityWarning, WasmAuthMethod, WasmPowerShellClient, WasmWinRmConfig,
    WinRmDestination,
};
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn security_check_allows_sspi_sealed_tcp_over_plain_gateway() {
    let cfg = test_config("ws://localhost:7171", GatewayTransport::Tcp, None);

    assert!(cfg.check_security().is_empty());
    assert_eq!(WasmPowerShellClient::check_security(&cfg).length(), 0);
}

#[wasm_bindgen_test]
fn security_check_reports_plain_gateway_to_tls_destination() {
    let cfg = test_config("ws://localhost:7171", GatewayTransport::Tls, None);

    assert_eq!(
        cfg.check_security(),
        vec![SecurityWarning::GatewayChannelInsecure]
    );
    assert_eq!(WasmPowerShellClient::check_security(&cfg).length(), 1);
}

fn test_config(
    gateway_url: impl Into<String>,
    transport: GatewayTransport,
    force_insecure: Option<bool>,
) -> WasmWinRmConfig {
    WasmWinRmConfig {
        auth: WasmAuthMethod::Basic,
        destination: WinRmDestination {
            host: "127.0.0.1".to_string(),
            port: 5985,
            transport,
        },
        gateway_url: gateway_url.into(),
        gateway_token: "token".to_string(),
        username: "user".to_string(),
        password: "pass".to_string(),
        domain: None,
        locale: None,
        kdc_proxy_url: None,
        client_computer_name: None,
        cols: 120,
        rows: 30,
        raw_ui_enabled: Some(true),
        force_insecure,
        configuration_name: None,
    }
}
