# CI Hardening, TLS Options, JEA ConfigurationName, Disconnect/Reconnect — Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Close the CI/test gap (tests never run in CI, connector untestable without a live server) and ship three features: TLS validation options for HTTPS WinRM, JEA `ConfigurationName` support, and PSRP session Disconnect/Reconnect (including reattach from a new client).

**Architecture:** The connector in `ironposh-client-core` is a sans-IO state machine (`Connector::step` consumes `Option<HttpResponseTargeted>`, returns requests to send). We exploit that: Phase 1 builds a fixture-driven "fake server" test harness that drives the state machine with programmatically built SOAP/PSRP responses — no network. Every later feature lands test-first against that harness. TLS options live in `ironposh-client-core` config but are *honored* by each `HttpClient` implementation (the trait at `crates/ironposh-async/src/lib.rs:79`), since TLS happens in reqwest/fetch, not in the protocol core. JEA is a one-field plumb into the `WsMan` builder. Disconnect/Reconnect adds new WS-Management actions in `ironposh-winrm`, new state transitions in `RunspacePool`/`ActiveSession`, and a new connector entry path (`connect to existing shell`) for browser-refresh-style reattach.

**Tech Stack:** Rust workspace (edition 2021/2024), `thiserror` per protocol crate / `anyhow` in clients, `tracing` structured logging (always `info!(?dbg_var, name = %fmt_var, "msg")` style), `typed_builder`, `reqwest`, `sspi`. Dev-deps added by this plan: `rcgen`, `tokio-rustls` (TLS test server).

**Scope guards:**
- The sync client (`ironposh-client-sync`) intentionally does NOT get the new features. It already lags (no gateway, no KDC proxy). Do not touch it. Note this divergence in README if asked.
- All work assumes the currently uncommitted gateway/KDC changes are landed first (they touch the same files: `config.rs`, `http_client.rs` in the tokio client). **Precondition: clean working tree.**
- Per repo rules: run `cargo clippy` iteratively after each change; do NOT run `cargo build` unless asked; never commit with a Claude author.

**Reference material:**
- MS-WSMV §3.1.4.13 (Disconnect), §3.1.4.14 (Reconnect), §3.1.4.15 (Connect)
- MS-PSRP §2.2.2.14 (CONNECT_RUNSPACEPOOL), §3.1.5.4 (reconnection sequence)
- Reference implementation: pypsrp — `pypsrp/wsman.py` (`disconnect()`, `reconnect()`, `connect()`) and `psrp/_connection/wsman.py`
- Existing fixture style: `crates/ironposh-winrm/tests/test_parse_resource_created.rs` + `tests/resources/*.xml`
- E2E server config helper: `crates/ironposh-client-tokio/tests/support/e2e_pwsh_config.rs` (env vars `IRONPOSH_E2E_SERVER` / `E2E_PWSH_*`, with reachable lab defaults)

---

## Phase 1 — Test foundation & CI

### Task 1: Make `clippy --all-targets` pass

AGENTS.md claims CI requires `cargo clippy --all-targets --all-features` but `ci.yml:39` only checks `--lib --bins`. Tests/examples may have drifted.

**Files:**
- Modify: whatever clippy flags (tests, examples) — discover via the command below.

**Step 1: Run clippy across all targets**

```powershell
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

**Step 2: Fix every warning** (mechanical; match existing code style, no drive-by refactors).

**Step 3: Re-run until clean.**

**Step 4: Commit**

```powershell
git add -A
git commit -m "chore: fix clippy warnings across all targets"
```

### Task 2: CI actually runs tests

**Files:**
- Modify: `C:\DevDrive\ironwinrm\.github\workflows\ci.yml` (rust job, around lines 36-39)

**Step 1: Widen clippy and add a test step.** In the rust job, change the clippy line and add a test step after it:

```yaml
      - name: Clippy
        run: cargo clippy --workspace --all-features --all-targets -- -D warnings

      - name: Test
        run: cargo test --workspace --exclude ironposh-web
```

Notes:
- `ironposh-web` is a wasm32 cdylib; it is built/tested by the existing wasm job, excluding it keeps the host test run green.
- All e2e tests are `#[ignore]`-gated, so `cargo test --workspace` only runs unit/integration tests that need no server. Verify with: `cargo test --workspace --exclude ironposh-web` locally — expect PASS with many `ignored`.

**Step 2: Verify the workflow file parses** (`yamllint`-by-eye or push to a branch and watch the run).

**Step 3: Commit**

```powershell
git add .github/workflows/ci.yml
git commit -m "ci: run workspace tests and clippy all targets"
```

### Task 3 (refactor): Fix the `conntion_pool` typo module

The central module of the connector is misspelled (`conntion_pool.rs`), which taxes every reader and every grep. Pure mechanical rename.

**Files:**
- Rename: `crates\ironposh-client-core\src\connector\conntion_pool.rs` → `connection_pool.rs`
- Modify: `crates\ironposh-client-core\src\connector\mod.rs:15,29` and every `conntion_pool::` path (search the workspace: `grep -r conntion_pool crates/`)

**Step 1:** `git mv crates/ironposh-client-core/src/connector/conntion_pool.rs crates/ironposh-client-core/src/connector/connection_pool.rs`

**Step 2:** Update `pub mod conntion_pool;` → `pub mod connection_pool;` and add a deprecated re-export ONLY if external crates reference the old path — check first: `grep -rn "conntion_pool" crates/ web/`. Fix all references (ironposh-async and clients import `TrySend` etc. through these paths).

**Step 3:** `cargo clippy --workspace --all-targets -- -D warnings` → clean. `cargo test --workspace --exclude ironposh-web` → PASS.

**Step 4: Commit**

```powershell
git add -A
git commit -m "refactor: rename conntion_pool module to connection_pool"
```

### Task 4: Connector handshake test harness (the foundation)

A fixture-driven fake server that drives `Connector::step` to `Connected` with zero network. Use `Basic` auth + `TransportSecurity::HttpInsecure` so bodies stay plaintext XML (no SSPI, no encryption). This harness is reused by Tasks 9, 14, 15, 20.

**Files:**
- Create: `crates\ironposh-client-core\tests\support\mod.rs`
- Create: `crates\ironposh-client-core\tests\connector_handshake.rs`

**Step 1: Write the harness + first failing test.**

`tests/support/mod.rs`:

```rust
//! Fake-server harness: drives the sans-IO Connector with canned HTTP responses.
use ironposh_client_core::connector::{
    Connector, ConnectorStepResult, TransportSecurity, WinRmConfig,
    config::AuthenticatorConfig,
    connection_pool::TrySend,
    http::{HttpBody, HttpRequest, HttpResponse, HttpResponseTargeted, ServerAddress},
};
use ironposh_psrp::HostInfo;

pub fn test_config() -> WinRmConfig {
    WinRmConfig {
        server: (ServerAddress::parse("127.0.0.1").unwrap(), 5985),
        transport: TransportSecurity::HttpInsecure,
        authentication: AuthenticatorConfig::Basic {
            username: "user".into(),
            password: "pass".into(),
        },
        host_info: HostInfo::default(),
        operation_timeout_secs: Some(1.0),
        // new fields added by later tasks get defaults here
    }
}

/// Extract (request, connection_id) from a TrySend, panicking on the auth path
/// (Basic auth never needs the SSPI KDC loop).
pub fn expect_just_send(try_send: TrySend) -> (HttpRequest, ironposh_client_core::connector::connection_pool::ConnectionId) {
    match try_send {
        TrySend::JustSend(action) => (action.request, action.connection_id),
        other => panic!("expected JustSend, got {other:?}"),
    }
}

/// Build a 200 response carrying `xml`, targeted back at `conn_id`.
pub fn xml_response(
    conn_id: ironposh_client_core::connector::connection_pool::ConnectionId,
    xml: String,
) -> HttpResponseTargeted {
    HttpResponseTargeted::new(
        HttpResponse {
            status_code: 200,
            headers: vec![],
            body: HttpBody::Xml(xml),
        },
        conn_id,
        None,
    )
}
```

> **Adaptation note for the implementer:** the exact variant names of `TrySend` and visibility of `ConnectionId` live in `connection_pool.rs` (post Task-3 rename). If `TrySend::JustSend` differs (check around `crates/ironposh-client-core/src/connector/connection_pool.rs:190-240`), adjust the matcher — do not change production visibility unless a getter is genuinely missing; if one is missing, add a minimal `pub fn` accessor, never `pub` fields.

`tests/connector_handshake.rs`, first test:

```rust
mod support;

use ironposh_client_core::connector::{Connector, ConnectorStepResult};

/// Idle step must emit the shell Create envelope with Basic auth preformatted.
#[test]
fn idle_step_emits_shell_create() {
    let mut connector = Connector::new(support::test_config());
    let result = connector.step(None).expect("idle step");

    let ConnectorStepResult::SendBack { try_send } = result else {
        panic!("expected SendBack from idle step");
    };
    let (request, _conn) = support::expect_just_send(try_send);

    let body = request.body.expect("create has a body");
    let xml = body.as_str().expect("plaintext body in HttpInsecure mode");
    assert!(xml.contains("http://schemas.xmlsoap.org/ws/2004/09/transfer/Create"));
    assert!(xml.contains("http://schemas.microsoft.com/powershell/Microsoft.PowerShell"));
    let auth = request.headers.iter().find(|(k, _)| k == "Authorization");
    assert!(auth.is_some(), "Basic auth header must be present");
}
```

**Step 2: Run it; make it compile and pass.**

```powershell
cargo test -p ironposh-client-core --test connector_handshake
```

Expected: first FAIL (compile errors on harness imports), iterate until PASS. This task is mostly *discovering the real public surface*; where the test can't reach something (e.g. `ConnectionId` not exported), add the smallest possible `pub use`/accessor in `ironposh-client-core`.

**Step 3: Add the full-handshake test.** The server side of the handshake is: `CreateResponse` (fixture exists: `crates/ironposh-winrm/tests/resources/resource_created.xml` — copy it to `crates/ironposh-client-core/tests/resources/`), then ReceiveResponses carrying PSRP `SessionCapability`, `ApplicationPrivateData`, `RunspacePoolState(Opened)` messages. Build those ReceiveResponse bodies *programmatically* with `ironposh_psrp::fragmentation::Fragmenter` + `ironposh-winrm` `ReceiveResponseValue` builders — exactly the pattern already proven in `crates/ironposh-client-core/tests/test_send_roundtrip.rs:39-107` (fragment → base64 → `Tag<Text, Stream>` → `ReceiveResponseValue` → SOAP envelope → `to_xml_string()`). The `rpid` must match the one the client generated — parse it out of the client's Create request body (`<creationXml>`... contains the SessionCapability the client sent; simpler: extract the shell id from your own CreateResponse fixture and the RPID from the client's first request envelope).

Test skeleton:

```rust
#[test]
fn handshake_reaches_connected() {
    let mut connector = Connector::new(support::test_config());

    // 1. Idle -> Create request
    let r1 = connector.step(None).unwrap();
    let (req1, conn1) = support::expect_just_send(unwrap_sendback(r1));
    let rpid = extract_rpid(req1.body.unwrap().as_str().unwrap()); // parse RPID from creationXml

    // 2. CreateResponse -> client fires Receive
    let create_resp = include_str!("resources/resource_created.xml");
    let r2 = connector.step(Some(support::xml_response(conn1, create_resp.into()))).unwrap();
    let (_req2, conn2) = support::expect_just_send(unwrap_sendback(r2));

    // 3. ReceiveResponse #1: SessionCapability + ApplicationPrivateData + RunspacePoolState(Opened)
    let recv1 = support::receive_response_with_psrp(rpid, &[
        psrp_session_capability(), psrp_application_private_data(), psrp_pool_state_opened(),
    ]);
    let r3 = connector.step(Some(support::xml_response(conn2, recv1))).unwrap();

    // 4. Depending on how the connector splits NegotiationSent/Opened, feed one more
    //    ReceiveResponse until we get Connected. Assert final state:
    assert!(matches!(r3, ConnectorStepResult::Connected { .. }));
}
```

> The connector transitions on `RunspacePoolState::NegotiationSent` vs `Opened` (`crates/ironposh-client-core/src/connector/mod.rs:307-332`). If the server messages must arrive across two Receive cycles (capability first, then state), split the PSRP messages across two responses. Iterate with `RUST_LOG=debug` until the state machine is satisfied; the error messages name the expected state. **If wire details are unclear, capture a real exchange** by running the existing ignored e2e (`cargo test -p ironposh-client-tokio --test command_matrix_e2e -- --ignored`) with `RUST_LOG=info` — the connector logs full outgoing/incoming XML (`shell_creation_xml`, `connecting_receive_xml`).

**Step 4: Run until green, clippy clean.**

**Step 5: Commit**

```powershell
git add crates/ironposh-client-core/tests
git commit -m "test: fixture-driven connector handshake harness (no network)"
```

---

## Phase 2 — Feature: TLS options for HTTPS WinRM

Real WinRM-over-HTTPS deployments overwhelmingly use self-signed certs. Today there is no knob at all — HTTPS is unusable against them.

### Task 5: `TlsOptions` type in client-core

**Files:**
- Modify: `crates\ironposh-client-core\src\connector\config.rs`
- Modify: `crates\ironposh-client-core\src\connector\mod.rs:83-102` (`WinRmConfig`)

**Step 1: Failing test** (in `config.rs` `#[cfg(test)]`):

```rust
#[test]
fn tls_options_default_is_secure() {
    let tls = TlsOptions::default();
    assert!(!tls.accept_invalid_certs);
    assert!(!tls.accept_invalid_hostnames);
    assert!(tls.extra_ca_pem.is_none());
}
```

**Step 2:** `cargo test -p ironposh-client-core tls_options` → FAIL (type missing).

**Step 3: Implement** in `config.rs`:

```rust
/// TLS behaviour for HTTPS transports. Honored by `HttpClient` implementations
/// (reqwest-based clients); ignored for plain-HTTP transports and for the WASM
/// client (the browser owns TLS there).
#[derive(Debug, Clone, Default)]
pub struct TlsOptions {
    /// Accept any server certificate (self-signed labs). DANGEROUS outside test/lab use.
    pub accept_invalid_certs: bool,
    /// Skip hostname verification only.
    pub accept_invalid_hostnames: bool,
    /// Additional root CA certificate(s), PEM-encoded.
    pub extra_ca_pem: Option<Vec<u8>>,
}
```

Add to `WinRmConfig`: `pub tls: TlsOptions,` (update the harness `test_config()` and every `WinRmConfig { .. }` construction site — compiler will list them: tokio client `config.rs`, web `conversions.rs`, sync client. For sync client just pass `TlsOptions::default()`).

**Step 4:** `cargo test -p ironposh-client-core` PASS; `cargo clippy --workspace --all-targets -- -D warnings` clean (this flushes out all construction sites).

**Step 5: Commit** — `feat(client-core): add TlsOptions to WinRmConfig`

### Task 6: reqwest client honors `TlsOptions`

**Files:**
- Modify: `crates\ironposh-client-tokio\src\http_client.rs` (`ReqwestHttpClient` constructor)
- Test: same file, `#[cfg(test)]` module

**Step 1: Failing tests:**

```rust
#[cfg(test)]
mod tls_tests {
    use super::*;
    use ironposh_client_core::connector::config::TlsOptions;

    #[test]
    fn builds_with_default_options() {
        assert!(build_reqwest_client(&TlsOptions::default()).is_ok());
    }

    #[test]
    fn builds_with_insecure_options() {
        let tls = TlsOptions { accept_invalid_certs: true, ..Default::default() };
        assert!(build_reqwest_client(&tls).is_ok());
    }

    #[test]
    fn rejects_garbage_ca_pem() {
        let tls = TlsOptions { extra_ca_pem: Some(b"not a pem".to_vec()), ..Default::default() };
        assert!(build_reqwest_client(&tls).is_err());
    }
}
```

**Step 2:** Run → FAIL (`build_reqwest_client` missing).

**Step 3: Implement:**

```rust
pub fn build_reqwest_client(tls: &TlsOptions) -> anyhow::Result<reqwest::Client> {
    let mut builder = reqwest::Client::builder()
        .danger_accept_invalid_certs(tls.accept_invalid_certs)
        .danger_accept_invalid_hostnames(tls.accept_invalid_hostnames);
    if let Some(pem) = &tls.extra_ca_pem {
        let cert = reqwest::Certificate::from_pem(pem).context("invalid extra CA PEM")?;
        builder = builder.add_root_certificate(cert);
    }
    builder.build().context("failed to build reqwest client")
}
```

Change `ReqwestHttpClient::new(...)` to take `&TlsOptions` (or a `with_tls` constructor) and use this function; keep the existing no-arg path delegating to defaults so untouched call sites keep compiling. Check reqwest feature flags in `crates/ironposh-client-tokio/Cargo.toml` — `danger_accept_invalid_hostnames` needs a TLS backend that supports it (`native-tls` or `rustls-tls`); if the current feature set doesn't expose it, drop hostname-only mode and keep just `accept_invalid_certs` + `extra_ca_pem` (YAGNI — don't add a backend for it).

**Step 4:** `cargo test -p ironposh-client-tokio tls_tests` PASS, clippy clean.

**Step 5: Commit** — `feat(client-tokio): TlsOptions-aware reqwest client construction`

### Task 7: Integration test against a real self-signed TLS listener

Proves the option changes actual connection behavior, not just builder plumbing.

**Files:**
- Modify: `crates\ironposh-client-tokio\Cargo.toml` (dev-deps: `rcgen`, `tokio-rustls`, `rustls-pemfile` if needed)
- Create: `crates\ironposh-client-tokio\tests\tls_options.rs`

**Step 1: Failing test:** spawn a localhost `tokio-rustls` acceptor with an rcgen self-signed cert that answers any TLS connection with a minimal `HTTP/1.1 401 Unauthorized` response, then:

```rust
#[tokio::test]
async fn default_tls_rejects_self_signed() {
    let (addr, _handle) = spawn_self_signed_https_server().await;
    let client = build_reqwest_client(&TlsOptions::default()).unwrap();
    let err = client.post(format!("https://{addr}/wsman")).send().await.unwrap_err();
    assert!(err.is_connect(), "expected TLS failure, got {err:?}");
}

#[tokio::test]
async fn insecure_tls_accepts_self_signed() {
    let (addr, _handle) = spawn_self_signed_https_server().await;
    let tls = TlsOptions { accept_invalid_certs: true, ..Default::default() };
    let client = build_reqwest_client(&tls).unwrap();
    let resp = client.post(format!("https://{addr}/wsman")).send().await.unwrap();
    assert_eq!(resp.status(), 401); // reached the server through TLS
}

#[tokio::test]
async fn extra_ca_pem_trusts_custom_ca() {
    let (addr, ca_pem, _handle) = spawn_https_server_with_ca().await; // rcgen CA + leaf for "localhost"
    let tls = TlsOptions { extra_ca_pem: Some(ca_pem), ..Default::default() };
    let client = build_reqwest_client(&tls).unwrap();
    assert_eq!(client.post(format!("https://localhost:{port}/wsman", port = addr.port()))
        .send().await.unwrap().status(), 401);
}
```

(`spawn_self_signed_https_server` is ~40 lines: rcgen cert for `localhost`, `tokio_rustls::TlsAcceptor`, accept loop writing a canned 401 with `Content-Length: 0`. These are normal tests — localhost only, no `#[ignore]`.)

**Step 2:** Run → FAIL. **Step 3:** Implement the helper. **Step 4:** PASS + clippy. **Step 5: Commit** — `test(client-tokio): TLS option behavior against self-signed listener`

### Task 8: CLI flags `--insecure` / `--ca-cert`

**Files:**
- Modify: `crates\ironposh-client-tokio\src\config.rs` (clap args struct + the `create_connector_config*` functions + existing config tests)
- Modify: `crates\ironposh-client-tokio\src\main.rs` (pass TlsOptions into `ReqwestHttpClient` / gateway path ignores it)

**Step 1: Failing test** (the config tests in `config.rs` already construct `Args`; extend):

```rust
#[test]
fn insecure_flag_maps_to_tls_options() {
    let args = parse_args_from(["prog", "-s", "1.2.3.4", "-u", "u", "-P", "p", "--https", "--insecure"]);
    let tls = tls_options_from(&args).unwrap();
    assert!(tls.accept_invalid_certs);
}

#[test]
fn ca_cert_flag_reads_pem_file() { /* tempfile with a valid rcgen PEM; assert extra_ca_pem is Some */ }

#[test]
fn insecure_without_https_is_rejected() {
    // --insecure with plain HTTP is meaningless; fail fast with a clear error
    assert!(validate_tls_flags(/* https: */ false, /* insecure: */ true).is_err());
}
```

**Step 2-4:** clap fields `insecure: bool`, `ca_cert: Option<PathBuf>`; implement `tls_options_from`; thread into `WinRmConfig.tls` and `ReqwestHttpClient`. Run config tests; clippy.

**Step 5: Commit** — `feat(client-tokio): --insecure and --ca-cert flags for HTTPS WinRM`

---

## Phase 3 — Feature: JEA `ConfigurationName`

The shell resource URI is already parameterizable in `WsMan` (`crates/ironposh-winrm/src/ws_management/mod.rs:34-35` defaults to `.../powershell/Microsoft.PowerShell`) — but the connector never sets it (`connector/mod.rs:223-228`). This is pure plumbing.

### Task 9: `configuration_name` on `WinRmConfig`, threaded into `WsMan`

**Files:**
- Modify: `crates\ironposh-client-core\src\connector\mod.rs` (`WinRmConfig` + the `Idle` arm of `step`)
- Test: `crates\ironposh-client-core\tests\connector_handshake.rs` (harness from Task 4)

**Step 1: Failing test:**

```rust
#[test]
fn configuration_name_sets_shell_resource_uri() {
    let mut config = support::test_config();
    config.configuration_name = Some("MyJEAEndpoint".into());
    let mut connector = Connector::new(config);

    let r = connector.step(None).unwrap();
    let (req, _) = support::expect_just_send(unwrap_sendback(r));
    let xml = req.body.unwrap().as_str().unwrap().to_owned();
    assert!(xml.contains("http://schemas.microsoft.com/powershell/MyJEAEndpoint"));
    assert!(!xml.contains("powershell/Microsoft.PowerShell"));
}
```

**Step 2:** Run → FAIL (field missing).

**Step 3: Implement.** On `WinRmConfig`:

```rust
/// PowerShell session configuration (JEA endpoint) name.
/// `None` → `Microsoft.PowerShell`. Becomes the shell resource URI
/// `http://schemas.microsoft.com/powershell/{name}`.
pub configuration_name: Option<String>,
```

plus a helper and the plumb in the `Idle` arm:

```rust
impl WinRmConfig {
    pub fn shell_resource_uri(&self) -> String {
        format!(
            "http://schemas.microsoft.com/powershell/{}",
            self.configuration_name.as_deref().unwrap_or("Microsoft.PowerShell")
        )
    }
}
// Idle arm:
let ws_man = Arc::new(
    WsMan::builder()
        .to(self.config.wsman_to(None))
        .operation_timeout(operation_timeout)
        .resource_uri(self.config.shell_resource_uri())
        .build(),
);
```

Fix all `WinRmConfig` construction sites (`configuration_name: None`) — compiler-driven, includes web `conversions.rs` and both CLI configs.

**Step 4:** Harness tests PASS, clippy clean across workspace.

**Step 5: Commit** — `feat(client-core): JEA ConfigurationName support via shell resource URI`

### Task 10: Expose it — tokio CLI flag + web config field

**Files:**
- Modify: `crates\ironposh-client-tokio\src\config.rs` (clap arg `--configuration-name`, map into `WinRmConfig`, extend config tests)
- Modify: `crates\ironposh-web\src\conversions.rs` (+ its test; add optional `configurationName` to the JS-facing config type where the other session options live — follow how `operation_timeout_secs` flows)
- Create: `crates\ironposh-client-tokio\tests\configuration_name_e2e.rs`

**Step 1: Failing config test** — `--configuration-name Foo` lands in `WinRmConfig.configuration_name`.

**Step 2-3:** Implement flag + plumb; mirror in web conversions (`configuration_name: input.configuration_name.clone()`), update the conversions unit test.

**Step 4: e2e proof (ignored test):** connect with `--configuration-name Microsoft.PowerShell` explicitly (works against any server — proves the URI is accepted end-to-end without needing a JEA endpoint), run `whoami`, expect success. Reuse `tests/support/e2e_pwsh_config.rs` + the subprocess pattern from `command_matrix_e2e.rs` (`Command::new(env!("CARGO_BIN_EXE_ironposh-client-tokio"))`). Run: `cargo test -p ironposh-client-tokio --test configuration_name_e2e -- --ignored`.

**Step 5: Commit** — `feat: expose configuration name in tokio CLI and web config`

---

## Phase 4 — Feature: Disconnect/Reconnect (same client)

WS-Management level first (new actions + bodies in `ironposh-winrm`, TDD on exact XML like the existing tests there), then state transitions in client-core, then client exposure. The PSRP-level states already exist (`crates/ironposh-client-core/src/runspace_pool/enums.rs:47` `Disconnected = 9` etc.) — only operations are missing.

### Task 11: `WsAction::Disconnect` / `Reconnect` envelopes

**Files:**
- Modify: `crates\ironposh-winrm\src\ws_management\mod.rs:50-80` (`WsAction`)
- Modify: `crates\ironposh-winrm\src\ws_management\body.rs` + `crates\ironposh-winrm\src\rsp\` (new `Disconnect` body element)
- Create: `crates\ironposh-winrm\tests\test_disconnect_request.rs`

**Step 1: Failing test** (mirror `test_initial_build_request.rs` style):

```rust
#[test]
fn disconnect_envelope_has_action_selector_and_body() {
    let ws_man = WsMan::builder().to("http://srv:5985/wsman".to_string()).build();
    let shell_id = uuid::Uuid::parse_str("11111111-2222-3333-4444-555555555555").unwrap();

    let xml = build_disconnect(&ws_man, shell_id, /* idle_timeout_secs */ None)
        .into_element().to_xml_string().unwrap();

    assert!(xml.contains("http://schemas.microsoft.com/wbem/wsman/1/windows/shell/Disconnect"));
    assert!(xml.contains("ShellId"));
    assert!(xml.contains("11111111-2222-3333-4444-555555555555"));
    assert!(xml.contains("<rsp:Disconnect"));
}

#[test]
fn reconnect_envelope_has_action_and_selector() { /* same shape, action .../shell/Reconnect, empty body */ }
```

**Step 2:** FAIL. **Step 3: Implement:**

- `WsAction::Disconnect` → `"http://schemas.microsoft.com/wbem/wsman/1/windows/shell/Disconnect"`, `WsAction::Reconnect` → `".../shell/Reconnect"` (extend the `as_str` match at `ws_management/mod.rs:64-79`).
- `rsp:Disconnect` body value type in `rsp/` (optional `<rsp:IdleTimeOut>PT{n}S</rsp:IdleTimeOut>` child), wired into `SoapBody` like `send`/`receive` are.
- Selector: reuse the existing `SelectorSetValue` (`ws_management/header.rs`) with `Name="ShellId"` — the same mechanism `win_rs.rs` uses for per-shell operations (see `crates/ironposh-client-core/src/runspace/win_rs.rs:295-305` for a usage example).
- The response bodies (`DisconnectResponse`/`ReconnectResponse`) are empty-bodied SOAP envelopes — parsing just validates the action header; add a parse test with a literal XML string. **Capture a real response first**: against the lab server, run pypsrp's `wsman.disconnect()` or temporarily wire the new request into the tokio client behind a debug command and log the response XML (`RUST_LOG=info`). Save as `crates/ironposh-winrm/tests/resources/disconnect_response.xml`.

**Step 4:** `cargo test -p ironposh-winrm` PASS, clippy. **Step 5: Commit** — `feat(winrm): Disconnect/Reconnect WS-Management actions`

### Task 12: `RunspacePool::fire_disconnect` / `fire_reconnect` + state transitions

**Files:**
- Modify: `crates\ironposh-client-core\src\runspace_pool\pool.rs` (new methods near `fire_receive`)
- Test: `crates\ironposh-client-core\tests\connector_handshake.rs` (extend harness) or `#[cfg(test)]` in `pool.rs`

**Step 1: Failing tests:**

```rust
#[test]
fn disconnect_only_valid_when_opened() {
    // pool in Opened state (drive via harness to Connected, then take its pool—
    // or construct via the same path expect_shell_created uses)
    // fire_disconnect() -> Ok(xml), state == Disconnecting
    // fire_disconnect() again -> Err(InvalidState)
}

#[test]
fn disconnect_response_moves_to_disconnected_and_reconnect_restores() {
    // accept_disconnect_response() -> state Disconnected
    // fire_reconnect() -> Ok(xml), state Reconnecting
    // accept_reconnect_response() -> state Opened
}
```

**Step 2:** FAIL. **Step 3: Implement** the four methods. Each `fire_*` builds the envelope via the Task-11 builders (the pool already owns `Arc<WsMan>` and the shell id — see how `fire_receive` does it), sets the transitional state, and returns the XML `String` exactly like `fire_receive`. Each `accept_*` validates the response action and lands the final state. Use `PwshCoreError::InvalidState` for wrong-state calls — match the existing error style.

**Step 4:** PASS + clippy. **Step 5: Commit** — `feat(client-core): runspace pool disconnect/reconnect state transitions`

### Task 13: `ActiveSession`/client plumbing + e2e

**Files:**
- Modify: `crates\ironposh-client-core\src\connector\active_session.rs` (new `UserOperation::Disconnect` / `Reconnect` variants and handling; follow how an existing operation like pipeline-create flows through to `fire_*` + send)
- Modify: `crates\ironposh-async\src\` (surface as client method `disconnect()` → emits a session event with the shell id; follow the `send_script` pattern in `client.rs`)
- Modify: `crates\ironposh-client-tokio\src\repl.rs` (REPL meta-command, e.g. `:disconnect` — print the shell id so the user can reattach later)
- Create: `crates\ironposh-client-tokio\tests\disconnect_reconnect_e2e.rs` (`#[ignore]`)

**Step 1:** Unit-test the `ActiveSession` operation routing with the harness (operation → outgoing XML contains the Disconnect action).
**Step 2:** Implement through the layers (compiler-driven; each layer is a thin pass-through — keep it that way, no new abstractions).
**Step 3: e2e (ignored):** session A: run `$x = 42`, disconnect, reconnect, run `$x` → expect `42` (proves the runspace truly survived). Subprocess pattern from `command_matrix_e2e.rs`.
**Step 4:** `cargo test -p ironposh-client-tokio --test disconnect_reconnect_e2e -- --ignored` PASS against the lab server.
**Step 5: Commit** — `feat: session disconnect/reconnect through async client and REPL`

---

## Phase 5 — Feature: Connect to a disconnected shell from a NEW client (browser-refresh reattach)

This is the payoff feature (gateway/web terminal: refresh → reattach) and the largest. It needs the WSMan `Connect` action carrying PSRP `SESSION_CAPABILITY` + `CONNECT_RUNSPACEPOOL` (`MessageType::ConnectRunspacepool` already exists: `crates/ironposh-psrp/src/cores.rs:72`), and a second connector entry path.

### Task 14: `WsAction::Connect` + `rsp:Connect`/`ConnectResponse` bodies

Same TDD shape as Task 11. The Connect body carries a `connectXml`-style base64 payload of the fragmented PSRP messages (mirror how the Create path embeds `creationXml` — find it with `git grep -n creationXml crates/`). The `ConnectResponse` returns the server's PSRP response fragments. **Before implementing the parser, capture a real ConnectResponse** with pypsrp against the lab server (`python -c` script using `pypsrp.wsman` + `connect()`), save to `tests/resources/connect_response.xml`.

**Commit:** `feat(winrm): Connect action for disconnected shells`

### Task 15: PSRP `ConnectRunspacePool` message

**Files:** `crates\ironposh-psrp\src\messages\` (new message struct; follow any small existing message, e.g. the runspace-state message in `messages/runspace_pool_state.rs`), registered in the message enum/dispatch.

TDD: serialize test asserting exact CLIXML (`<Obj><MS><I32 N="MinRunspaces">…` per MS-PSRP §2.2.2.14 — verify the exact shape against pypsrp `pypsrp/messages.py::ConnectRunspacePool`), plus a roundtrip parse test.

**Commit:** `feat(psrp): ConnectRunspacePool message`

### Task 16: Connector reattach path

**Files:**
- Modify: `crates\ironposh-client-core\src\connector\mod.rs` (`Connector::new_connect(config, shell_id)` or a `ConnectorMode` enum on construction; new state arm `ConnectingExisting` between `Idle` and `ConnectReceiveCycle`)
- Test: harness (`connector_handshake.rs`): `connect_mode_emits_wsman_connect_with_connect_runspacepool` — assert first request has the Connect action, the ShellId selector, and a base64 payload that defragments back to `[SessionCapability, ConnectRunspacePool]` (use `Defragmenter` in the test, like `test_send_roundtrip.rs:141-148`).

Flow to implement (verify against MS-PSRP §3.1.5.4 / pypsrp `psrp/_connection/wsman.py`): send Connect(SESSION_CAPABILITY + CONNECT_RUNSPACEPOOL) → parse ConnectResponse fragments (SESSION_CAPABILITY + RUNSPACEPOOL_INIT_DATA) → enter the normal Receive cycle → `Connected` with a working `ActiveSession`. Reuse `ExpectShellCreated`'s sibling pattern — add `ExpectShellConnected` rather than overloading the existing type.

**Commit:** `feat(client-core): connector reattach path for disconnected shells`

### Task 17: CLI exposure + full reattach e2e

**Files:**
- Modify: `crates\ironposh-client-tokio\src\config.rs` (`--connect-shell-id <UUID>`), `main.rs` (construct connector in connect mode)
- Create: `crates\ironposh-client-tokio\tests\reattach_e2e.rs` (`#[ignore]`)

e2e: process A connects, runs `$marker = "alive-$(Get-Random)"`, prints shell id, disconnects, exits. Process B starts with `--connect-shell-id`, runs `$marker`, asserts the same value comes back. This single test proves the entire Phase 4+5 stack.

**Commit:** `feat(client-tokio): --connect-shell-id reattach support`

### Task 18 (follow-up, optional): web client reattach

Expose connect-mode through `ironposh-web` (shell id in/out of the JS config) so the browser terminal can survive refresh — the actual motivating use case. Scope it once Tasks 14–17 are proven; it's conversions + a constructor variant, no new protocol work.

---

## Execution order & checkpoints

| Order | Task | Why this order |
|---|---|---|
| 1-2 | clippy + CI tests | Everything after this is protected by CI |
| 3 | connection_pool rename | Before new code references the old path |
| 4 | Fake-server harness | Foundation for every feature test |
| 5-8 | TLS | Smallest feature, exercises the new harness + config plumb pattern |
| 9-10 | JEA | One-field plumb, builds on the same pattern |
| 11-13 | Disconnect/Reconnect | Protocol → core → client, each layer TDD |
| 14-17 | Reattach | Largest; depends on 11-13 |
| 18 | Web reattach | Optional follow-up |

**Checkpoint after each phase:** `cargo clippy --workspace --all-targets --all-features -- -D warnings` clean, `cargo test --workspace --exclude ironposh-web` green, then (when a lab server is available) the relevant `--ignored` e2e for the phase. Do not start a phase with the previous one uncommitted.
