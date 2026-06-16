//! Shared test-support code for the ironposh workspace (`publish = false`).
//!
//! Consumed as a dev-dependency only. Hosts:
//! - [`fake_server`]: sans-IO Connector harness driven by canned HTTP responses
//! - [`tls_listener`]: local self-signed TLS listener helpers
//! - [`e2e_pwsh_config`]: env-var loader for real-server e2e configuration
//! - [`pty_harness`] / [`native_pty_matrix`]: PTY-driven e2e harnesses

pub mod e2e_pwsh_config;
pub mod fake_server;
pub mod native_pty_matrix;
pub mod pty_harness;
pub mod tls_listener;
