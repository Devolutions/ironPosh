//! Library target exposing the HTTP client internals so integration tests
//! (e.g. `tests/tls_options.rs`) can exercise them. The binary in `main.rs`
//! compiles the same module directly via `mod http_client;`.

pub mod http_client;
