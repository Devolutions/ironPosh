use wasm_bindgen::prelude::*;

pub mod client;
pub mod conversions;
pub mod error;
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
