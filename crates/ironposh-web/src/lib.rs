use serde::{Deserialize, Serialize};
use tsify::Tsify;
use wasm_bindgen::prelude::*;

pub mod client;
pub mod conversions;
pub mod error;
pub mod hostcall;
pub mod http_client;
pub mod http_convert;
pub mod stream;
pub mod types;
pub mod websocket;

// Re-export the main types for JS/TS
pub use client::WasmPowerShellClient;
pub use stream::WasmPowerShellStream;
pub use types::*;

// When the `wee_alloc` feature is enabled, use `wee_alloc` as the global allocator.
#[cfg(feature = "wee_alloc")]
#[global_allocator]
static ALLOC: wee_alloc::WeeAlloc = wee_alloc::WeeAlloc::INIT;

#[wasm_bindgen]
pub fn set_panic_hook() {
    console_error_panic_hook::set_once();
}

#[derive(Tsify, Serialize, Deserialize, Debug, Clone)]
#[tsify(into_wasm_abi, from_wasm_abi)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

/// Initialize tracing for WASM with a custom max level
/// Levels: 1=ERROR, 2=WARN, 3=INFO, 4=DEBUG, 5=TRACE
#[wasm_bindgen]
pub fn init_tracing_with_level(max_level: LogLevel) {
    use tracing::Level;
    use tracing_wasm::WASMLayerConfigBuilder;

    let level = match max_level {
        LogLevel::Error => Level::ERROR,
        LogLevel::Warn => Level::WARN,
        LogLevel::Info => Level::INFO,
        LogLevel::Debug => Level::DEBUG,
        LogLevel::Trace => Level::TRACE,
    };

    let config = WASMLayerConfigBuilder::new().set_max_level(level).build();

    tracing_wasm::set_as_global_default_with_config(config);
}
