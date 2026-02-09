use std::convert::TryFrom;

use futures::StreamExt;
use ironposh_async::HostResponse;
use ironposh_client_core::host::{self as host, HostCall, Submission};
use ironposh_psrp::{PipelineHostResponse, PsPrimitiveValue, PsValue};
use js_sys::Promise;
use js_sys::{Object, Reflect, Uint8Array};
use tracing::{error, warn};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_futures::JsFuture;

use crate::JsHostCall;

async fn await_maybe_promise(promise: JsValue) -> Result<JsValue, JsValue> {
    match promise.dyn_into::<Promise>() {
        Ok(promise) => {
            let js_future = JsFuture::from(promise);
            js_future.await
        }
        Err(value) => Ok(value),
    }
}

/// Helper function to call JS handler and await result
async fn call_js_handler(
    handler: &js_sys::Function,
    this: &JsValue,
    params: &JsValue,
    method: &str,
) -> Result<JsValue, ()> {
    let value = handler.call1(this, params).map_err(|e| {
        error!(?e, method = method, "calling JS host call handler failed");
    })?;

    await_maybe_promise(value).await.map_err(|e| {
        error!(
            ?e,
            method = method,
            "awaiting JS host call handler promise failed"
        );
    })
}

fn exception_submission(
    call_id: i64,
    method_id: i32,
    method_name: &str,
    message: String,
) -> Submission {
    Submission::Send(PipelineHostResponse {
        call_id,
        method_id,
        method_name: method_name.to_string(),
        method_result: None,
        method_exception: Some(PsValue::from(message)),
    })
}

fn utf16le_bytes(s: &str) -> Vec<u8> {
    // PowerShell SecureString payload is UTF-16LE bytes.
    // (Encryption, if any, happens later in the PSRP pipeline.)
    s.encode_utf16()
        .flat_map(u16::to_le_bytes)
        .collect::<Vec<u8>>()
}

struct JsI32(i32);

impl TryFrom<&JsValue> for JsI32 {
    type Error = String;

    fn try_from(value: &JsValue) -> Result<Self, Self::Error> {
        value
            .as_f64()
            .ok_or_else(|| "expected number".to_string())
            .map(|n| Self(n as i32))
    }
}

struct SecureBytes(Vec<u8>);

impl TryFrom<JsValue> for SecureBytes {
    type Error = String;

    fn try_from(value: JsValue) -> Result<Self, Self::Error> {
        if let Some(s) = value.as_string() {
            return Ok(Self(utf16le_bytes(&s)));
        }

        if let Ok(u8a) = value.clone().dyn_into::<Uint8Array>() {
            return Ok(Self(u8a.to_vec()));
        }

        if value.is_object() {
            if let Ok(arr) = serde_wasm_bindgen::from_value::<Vec<u8>>(value) {
                return Ok(Self(arr));
            }
        }

        Err("expected SecureString input as string | Uint8Array | number[]".to_string())
    }
}

struct JsChar(char);

impl TryFrom<&str> for JsChar {
    type Error = String;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        value
            .chars()
            .next()
            .map(Self)
            .ok_or_else(|| "expected single-character string".to_string())
    }
}

fn ps_value_from_js(v: JsValue) -> Result<PsValue, String> {
    if v.is_null() || v.is_undefined() {
        return Ok(PsValue::Primitive(PsPrimitiveValue::Nil));
    }

    if let Some(b) = v.as_bool() {
        return Ok(PsValue::Primitive(PsPrimitiveValue::Bool(b)));
    }

    if let Some(s) = v.as_string() {
        return Ok(PsValue::Primitive(PsPrimitiveValue::Str(s)));
    }

    if let Some(n) = v.as_f64() {
        if n.fract() == 0.0 {
            // Integer: prefer i32 when possible.
            let as_i64 = n as i64;
            if let Ok(as_i32) = i32::try_from(as_i64) {
                return Ok(PsValue::Primitive(PsPrimitiveValue::I32(as_i32)));
            }
            return Ok(PsValue::Primitive(PsPrimitiveValue::I64(as_i64)));
        }

        // No float primitive type yet in PsPrimitiveValue; fallback to string.
        return Ok(PsValue::Primitive(PsPrimitiveValue::Str(n.to_string())));
    }

    // Allow SecureString wrapper objects: { secureString: ... } / { SecureString: ... }.
    if v.is_object() {
        if let Ok(ss) = Reflect::get(&v, &JsValue::from_str("secureString")) {
            if !ss.is_undefined() {
                let bytes = SecureBytes::try_from(ss)?.0;
                return Ok(PsValue::Primitive(PsPrimitiveValue::SecureString(bytes)));
            }
        }
        if let Ok(ss) = Reflect::get(&v, &JsValue::from_str("SecureString")) {
            if !ss.is_undefined() {
                let bytes = SecureBytes::try_from(ss)?.0;
                return Ok(PsValue::Primitive(PsPrimitiveValue::SecureString(bytes)));
            }
        }
    }

    // Prefer structured JsPsValue if provided.
    if v.is_object() {
        if let Ok(js_ps_value) = serde_wasm_bindgen::from_value::<crate::JsPsValue>(v.clone()) {
            return PsValue::try_from(js_ps_value)
                .map_err(|e| format!("invalid JsPsValue from handler: {e}"));
        }
    }

    // Best-effort: allow downstream to provide full PsValue JSON shape.
    serde_wasm_bindgen::from_value::<PsValue>(v).map_err(|e| {
        format!(
            "unsupported prompt value; expected primitive | {{secureString: ...}} | PsValue JSON shape ({e})"
        )
    })
}

async fn submit_void(
    host_call_handler: &js_sys::Function,
    this: &JsValue,
    js_params: &JsValue,
    method_name: &str,
) {
    if call_js_handler(host_call_handler, this, js_params, method_name)
        .await
        .is_err()
    {
        warn!(
            method = method_name,
            "host call handler errored (void method)"
        );
    }
}

/// This should definately be handled by JS side, but for now we leave it like this so at least the session's loop is not blocked
#[expect(clippy::too_many_lines)]
pub async fn handle_host_calls(
    mut host_call_rx: futures::channel::mpsc::UnboundedReceiver<HostCall>,
    submitter: ironposh_async::HostSubmitter,
    host_call_handler: js_sys::Function,
) {
    while let Some(host_call) = host_call_rx.next().await {
        let scope = host_call.scope();
        let call_id = host_call.call_id();
        let method_name = host_call.method_name();
        let method_id = host_call.method_id();

        let js_host_call: JsHostCall = (&host_call).into();
        let this = JsValue::NULL;
        let js_params = JsValue::from(js_host_call);
        // let result = host_call_handler.call1(&this, &js_params)?;
        let submission = match host_call {
            // ===== Methods returning String =====
            HostCall::GetName { transport } => {
                let ((), rt) = transport.into_parts();
                match call_js_handler(&host_call_handler, &this, &js_params, method_name).await {
                    Ok(res) => match res.as_string() {
                        Some(value) => rt.accept_result(value),
                        None => rt.accept_result("IronPoshWebHost".to_string()),
                    },
                    Err(()) => rt.accept_result("IronPoshWebHost".to_string()),
                }
            }
            HostCall::GetVersion { transport } => {
                let ((), rt) = transport.into_parts();
                match call_js_handler(&host_call_handler, &this, &js_params, method_name).await {
                    Ok(res) => match res.as_string() {
                        Some(value) => rt.accept_result(value),
                        None => rt.accept_result("1.0.0".to_string()),
                    },
                    Err(()) => rt.accept_result("1.0.0".to_string()),
                }
            }
            HostCall::GetCurrentCulture { transport } => {
                let ((), rt) = transport.into_parts();
                match call_js_handler(&host_call_handler, &this, &js_params, method_name).await {
                    Ok(res) => match res.as_string() {
                        Some(value) => rt.accept_result(value),
                        None => rt.accept_result("en-US".to_string()),
                    },
                    Err(()) => rt.accept_result("en-US".to_string()),
                }
            }
            HostCall::GetCurrentUICulture { transport } => {
                let ((), rt) = transport.into_parts();
                match call_js_handler(&host_call_handler, &this, &js_params, method_name).await {
                    Ok(res) => match res.as_string() {
                        Some(value) => rt.accept_result(value),
                        None => rt.accept_result("en-US".to_string()),
                    },
                    Err(()) => rt.accept_result("en-US".to_string()),
                }
            }
            HostCall::ReadLine { transport } => {
                let ((), rt) = transport.into_parts();
                match call_js_handler(&host_call_handler, &this, &js_params, method_name).await {
                    Ok(res) => match res.as_string() {
                        Some(value) => rt.accept_result(value),
                        None => rt.accept_result(String::new()),
                    },
                    Err(()) => rt.accept_result(String::new()),
                }
            }
            HostCall::GetWindowTitle { transport } => {
                let ((), rt) = transport.into_parts();
                match call_js_handler(&host_call_handler, &this, &js_params, method_name).await {
                    Ok(res) => match res.as_string() {
                        Some(value) => rt.accept_result(value),
                        None => rt.accept_result(String::new()),
                    },
                    Err(()) => rt.accept_result(String::new()),
                }
            }

            // ===== Methods returning i32 =====
            HostCall::PromptForChoice { transport } => {
                let (_params, rt) = transport.into_parts();
                match call_js_handler(&host_call_handler, &this, &js_params, method_name).await {
                    Ok(res) => match JsI32::try_from(&res) {
                        Ok(v) => rt.accept_result(v.0),
                        Err(e) => exception_submission(call_id, method_id, method_name, e),
                    },
                    Err(()) => exception_submission(
                        call_id,
                        method_id,
                        method_name,
                        "PromptForChoice handler failed".to_string(),
                    ),
                }
            }
            HostCall::GetForegroundColor { transport } => {
                let ((), rt) = transport.into_parts();
                match call_js_handler(&host_call_handler, &this, &js_params, method_name).await {
                    Ok(res) => match res.as_f64().map(|v| v as i32) {
                        Some(v) => rt.accept_result(v),
                        None => rt.accept_result(7),
                    },
                    Err(()) => rt.accept_result(7),
                }
            }
            HostCall::GetBackgroundColor { transport } => {
                let ((), rt) = transport.into_parts();
                match call_js_handler(&host_call_handler, &this, &js_params, method_name).await {
                    Ok(res) => match res.as_f64().map(|v| v as i32) {
                        Some(v) => rt.accept_result(v),
                        None => rt.accept_result(0),
                    },
                    Err(()) => rt.accept_result(0),
                }
            }
            HostCall::GetCursorSize { transport } => {
                let ((), rt) = transport.into_parts();
                match call_js_handler(&host_call_handler, &this, &js_params, method_name).await {
                    Ok(res) => match JsI32::try_from(&res) {
                        Ok(v) => rt.accept_result(v.0),
                        Err(e) => exception_submission(call_id, method_id, method_name, e),
                    },
                    Err(()) => exception_submission(
                        call_id,
                        method_id,
                        method_name,
                        "GetCursorSize handler failed".to_string(),
                    ),
                }
            }

            // ===== Methods returning bool =====
            HostCall::GetKeyAvailable { transport } => {
                let ((), rt) = transport.into_parts();
                match call_js_handler(&host_call_handler, &this, &js_params, method_name).await {
                    Ok(res) => rt.accept_result(res.as_bool().unwrap_or(false)),
                    Err(()) => rt.accept_result(false),
                }
            }
            HostCall::GetIsRunspacePushed { transport } => {
                let ((), rt) = transport.into_parts();
                match call_js_handler(&host_call_handler, &this, &js_params, method_name).await {
                    Ok(res) => rt.accept_result(res.as_bool().unwrap_or(false)),
                    Err(()) => exception_submission(
                        call_id,
                        method_id,
                        method_name,
                        "GetIsRunspacePushed handler failed".to_string(),
                    ),
                }
            }

            // ===== Methods returning uuid::Uuid =====
            HostCall::GetInstanceId { transport } => {
                let ((), rt) = transport.into_parts();
                match call_js_handler(&host_call_handler, &this, &js_params, method_name).await {
                    Ok(res) => {
                        if let Some(uuid_str) = res.as_string() {
                            if let Ok(v) = uuid::Uuid::parse_str(&uuid_str) {
                                rt.accept_result(v)
                            } else {
                                rt.accept_result(uuid::Uuid::nil())
                            }
                        } else {
                            rt.accept_result(uuid::Uuid::nil())
                        }
                    }
                    Err(()) => rt.accept_result(uuid::Uuid::nil()),
                }
            }

            // ===== Methods returning Vec<u8> =====
            HostCall::ReadLineAsSecureString { transport } => {
                let ((), rt) = transport.into_parts();
                match call_js_handler(&host_call_handler, &this, &js_params, method_name).await {
                    Ok(res) => match SecureBytes::try_from(res) {
                        Ok(bytes) => rt.accept_result(bytes.0),
                        Err(e) => exception_submission(call_id, method_id, method_name, e),
                    },
                    Err(()) => exception_submission(
                        call_id,
                        method_id,
                        method_name,
                        "ReadLineAsSecureString handler failed".to_string(),
                    ),
                }
            }

            // ===== Void methods (no response expected) =====
            HostCall::SetShouldExit { transport: _ }
            | HostCall::EnterNestedPrompt { transport: _ }
            | HostCall::ExitNestedPrompt { transport: _ }
            | HostCall::NotifyBeginApplication { transport: _ }
            | HostCall::NotifyEndApplication { transport: _ }
            | HostCall::Write1 { transport: _ }
            | HostCall::Write2 { transport: _ }
            | HostCall::WriteLine1 { transport: _ }
            | HostCall::WriteLine2 { transport: _ }
            | HostCall::WriteLine3 { transport: _ }
            | HostCall::WriteErrorLine { transport: _ }
            | HostCall::WriteDebugLine { transport: _ }
            | HostCall::WriteProgress { transport: _ }
            | HostCall::WriteVerboseLine { transport: _ }
            | HostCall::WriteWarningLine { transport: _ }
            | HostCall::SetForegroundColor { transport: _ }
            | HostCall::SetBackgroundColor { transport: _ }
            | HostCall::SetCursorPosition { transport: _ }
            | HostCall::SetWindowPosition { transport: _ }
            | HostCall::SetCursorSize { transport: _ }
            | HostCall::SetBufferSize { transport: _ }
            | HostCall::SetWindowSize { transport: _ }
            | HostCall::SetWindowTitle { transport: _ }
            | HostCall::FlushInputBuffer { transport: _ }
            | HostCall::SetBufferContents1 { transport: _ }
            | HostCall::SetBufferContents2 { transport: _ }
            | HostCall::ScrollBufferContents { transport: _ }
            | HostCall::PushRunspace { transport: _ }
            | HostCall::PopRunspace { transport: _ } => {
                submit_void(&host_call_handler, &this, &js_params, method_name).await;
                Submission::NoSend
            }

            // ===== Not implemented (complex return types) =====
            HostCall::Prompt { transport } => {
                let (_params, rt) = transport.into_parts();
                match call_js_handler(&host_call_handler, &this, &js_params, method_name).await {
                    Ok(res) => {
                        let parsed: Result<std::collections::HashMap<String, PsValue>, String> =
                            (|| {
                                let obj = Object::from(res);
                                let keys = Object::keys(&obj);
                                let mut out = std::collections::HashMap::new();
                                for i in 0..keys.length() {
                                    let k = keys.get(i).as_string().unwrap_or_default();
                                    let vv = Reflect::get(&obj, &JsValue::from_str(&k)).map_err(
                                        |_| "failed to read prompt result key".to_string(),
                                    )?;
                                    let ps = ps_value_from_js(vv)?;
                                    out.insert(k, ps);
                                }
                                Ok(out)
                            })();

                        match parsed {
                            Ok(out) => rt.accept_result(out),
                            Err(e) => exception_submission(call_id, method_id, method_name, e),
                        }
                    }
                    Err(()) => exception_submission(
                        call_id,
                        method_id,
                        method_name,
                        "Prompt handler failed".to_string(),
                    ),
                }
            }
            HostCall::PromptForCredential1 { transport } => {
                let (_params, rt) = transport.into_parts();
                match call_js_handler(&host_call_handler, &this, &js_params, method_name).await {
                    Ok(res) => {
                        let cred = serde_wasm_bindgen::from_value::<crate::JsPSCredential>(res)
                            .map_err(|e| format!("invalid PSCredential from handler: {e}"));
                        match cred {
                            Ok(cred) => {
                                let password_bytes = match cred.password {
                                    crate::JsBytesOrString::Bytes(b) => b,
                                    crate::JsBytesOrString::Text(s) => utf16le_bytes(&s),
                                };
                                rt.accept_result(host::PSCredential {
                                    user_name: cred.user_name,
                                    password: password_bytes,
                                })
                            }
                            Err(e) => exception_submission(call_id, method_id, method_name, e),
                        }
                    }
                    Err(()) => exception_submission(
                        call_id,
                        method_id,
                        method_name,
                        "PromptForCredential1 handler failed".to_string(),
                    ),
                }
            }
            HostCall::PromptForCredential2 { transport } => {
                let (_params, rt) = transport.into_parts();
                match call_js_handler(&host_call_handler, &this, &js_params, method_name).await {
                    Ok(res) => {
                        let cred = serde_wasm_bindgen::from_value::<crate::JsPSCredential>(res)
                            .map_err(|e| format!("invalid PSCredential from handler: {e}"));
                        match cred {
                            Ok(cred) => {
                                let password_bytes = match cred.password {
                                    crate::JsBytesOrString::Bytes(b) => b,
                                    crate::JsBytesOrString::Text(s) => utf16le_bytes(&s),
                                };
                                rt.accept_result(host::PSCredential {
                                    user_name: cred.user_name,
                                    password: password_bytes,
                                })
                            }
                            Err(e) => exception_submission(call_id, method_id, method_name, e),
                        }
                    }
                    Err(()) => exception_submission(
                        call_id,
                        method_id,
                        method_name,
                        "PromptForCredential2 handler failed".to_string(),
                    ),
                }
            }
            HostCall::GetCursorPosition { transport } => {
                let ((), rt) = transport.into_parts();
                match call_js_handler(&host_call_handler, &this, &js_params, method_name).await {
                    Ok(res) => match serde_wasm_bindgen::from_value::<crate::JsCoordinates>(res) {
                        Ok(coords) => rt.accept_result(host::Coordinates::from(coords)),
                        Err(e) => exception_submission(
                            call_id,
                            method_id,
                            method_name,
                            format!("invalid Coordinates from handler: {e}"),
                        ),
                    },
                    Err(()) => exception_submission(
                        call_id,
                        method_id,
                        method_name,
                        "GetCursorPosition handler failed".to_string(),
                    ),
                }
            }
            HostCall::GetWindowPosition { transport } => {
                let ((), rt) = transport.into_parts();
                match call_js_handler(&host_call_handler, &this, &js_params, method_name).await {
                    Ok(res) => match serde_wasm_bindgen::from_value::<crate::JsCoordinates>(res) {
                        Ok(coords) => rt.accept_result(host::Coordinates::from(coords)),
                        Err(e) => exception_submission(
                            call_id,
                            method_id,
                            method_name,
                            format!("invalid Coordinates from handler: {e}"),
                        ),
                    },
                    Err(()) => exception_submission(
                        call_id,
                        method_id,
                        method_name,
                        "GetWindowPosition handler failed".to_string(),
                    ),
                }
            }
            HostCall::GetBufferSize { transport } => {
                let ((), rt) = transport.into_parts();
                match call_js_handler(&host_call_handler, &this, &js_params, method_name).await {
                    Ok(res) => match serde_wasm_bindgen::from_value::<crate::JsSize>(res) {
                        Ok(size) => rt.accept_result(host::Size::from(size)),
                        Err(e) => exception_submission(
                            call_id,
                            method_id,
                            method_name,
                            format!("invalid Size from handler: {e}"),
                        ),
                    },
                    Err(()) => exception_submission(
                        call_id,
                        method_id,
                        method_name,
                        "GetBufferSize handler failed".to_string(),
                    ),
                }
            }
            HostCall::GetWindowSize { transport } => {
                let ((), rt) = transport.into_parts();
                match call_js_handler(&host_call_handler, &this, &js_params, method_name).await {
                    Ok(res) => match serde_wasm_bindgen::from_value::<crate::JsSize>(res) {
                        Ok(size) => rt.accept_result(host::Size::from(size)),
                        Err(e) => exception_submission(
                            call_id,
                            method_id,
                            method_name,
                            format!("invalid Size from handler: {e}"),
                        ),
                    },
                    Err(()) => exception_submission(
                        call_id,
                        method_id,
                        method_name,
                        "GetWindowSize handler failed".to_string(),
                    ),
                }
            }
            HostCall::GetMaxWindowSize { transport } => {
                let ((), rt) = transport.into_parts();
                match call_js_handler(&host_call_handler, &this, &js_params, method_name).await {
                    Ok(res) => match serde_wasm_bindgen::from_value::<crate::JsSize>(res) {
                        Ok(size) => rt.accept_result(host::Size::from(size)),
                        Err(e) => exception_submission(
                            call_id,
                            method_id,
                            method_name,
                            format!("invalid Size from handler: {e}"),
                        ),
                    },
                    Err(()) => exception_submission(
                        call_id,
                        method_id,
                        method_name,
                        "GetMaxWindowSize handler failed".to_string(),
                    ),
                }
            }
            HostCall::GetMaxPhysicalWindowSize { transport } => {
                let ((), rt) = transport.into_parts();
                match call_js_handler(&host_call_handler, &this, &js_params, method_name).await {
                    Ok(res) => match serde_wasm_bindgen::from_value::<crate::JsSize>(res) {
                        Ok(size) => rt.accept_result(host::Size::from(size)),
                        Err(e) => exception_submission(
                            call_id,
                            method_id,
                            method_name,
                            format!("invalid Size from handler: {e}"),
                        ),
                    },
                    Err(()) => exception_submission(
                        call_id,
                        method_id,
                        method_name,
                        "GetMaxPhysicalWindowSize handler failed".to_string(),
                    ),
                }
            }
            HostCall::ReadKey { transport } => {
                let (_params, rt) = transport.into_parts();
                match call_js_handler(&host_call_handler, &this, &js_params, method_name).await {
                    Ok(res) => match serde_wasm_bindgen::from_value::<crate::JsKeyInfo>(res) {
                        Ok(k) => match JsChar::try_from(k.character.as_str()) {
                            Ok(ch) => rt.accept_result(host::KeyInfo {
                                virtual_key_code: k.virtual_key_code,
                                character: ch.0,
                                control_key_state: k.control_key_state,
                                key_down: k.key_down,
                            }),
                            Err(e) => exception_submission(call_id, method_id, method_name, e),
                        },
                        Err(e) => exception_submission(
                            call_id,
                            method_id,
                            method_name,
                            format!("invalid KeyInfo from handler: {e}"),
                        ),
                    },
                    Err(()) => exception_submission(
                        call_id,
                        method_id,
                        method_name,
                        "ReadKey handler failed".to_string(),
                    ),
                }
            }
            HostCall::GetBufferContents { transport } => {
                let (_params, rt) = transport.into_parts();
                match call_js_handler(&host_call_handler, &this, &js_params, method_name).await {
                    Ok(res) => {
                        let rows =
                            serde_wasm_bindgen::from_value::<Vec<Vec<crate::JsBufferCell>>>(res)
                                .map_err(|e| format!("invalid BufferCell[][] from handler: {e}"));

                        match rows {
                            Ok(rows) => {
                                let mut out: Vec<Vec<host::BufferCell>> =
                                    Vec::with_capacity(rows.len());
                                for row in rows {
                                    let mut out_row = Vec::with_capacity(row.len());
                                    for cell in row {
                                        let ch = JsChar::try_from(cell.character.as_str())
                                            .map(|v| v.0)
                                            .unwrap_or(' ');
                                        out_row.push(host::BufferCell {
                                            character: ch,
                                            foreground: cell.foreground,
                                            background: cell.background,
                                            flags: cell.flags,
                                        });
                                    }
                                    out.push(out_row);
                                }
                                rt.accept_result(out)
                            }
                            Err(e) => exception_submission(call_id, method_id, method_name, e),
                        }
                    }
                    Err(()) => exception_submission(
                        call_id,
                        method_id,
                        method_name,
                        "GetBufferContents handler failed".to_string(),
                    ),
                }
            }
            HostCall::GetRunspace { transport } => {
                let ((), rt) = transport.into_parts();
                match call_js_handler(&host_call_handler, &this, &js_params, method_name).await {
                    Ok(res) => match ps_value_from_js(res) {
                        Ok(v) => rt.accept_result(v),
                        Err(e) => exception_submission(call_id, method_id, method_name, e),
                    },
                    Err(()) => exception_submission(
                        call_id,
                        method_id,
                        method_name,
                        "GetRunspace handler failed".to_string(),
                    ),
                }
            }
            HostCall::PromptForChoiceMultipleSelection { transport } => {
                let (_params, rt) = transport.into_parts();
                match call_js_handler(&host_call_handler, &this, &js_params, method_name).await {
                    Ok(res) => match serde_wasm_bindgen::from_value::<Vec<i32>>(res) {
                        Ok(v) => rt.accept_result(v),
                        Err(e) => exception_submission(
                            call_id,
                            method_id,
                            method_name,
                            format!("invalid i32[] from handler: {e}"),
                        ),
                    },
                    Err(()) => exception_submission(
                        call_id,
                        method_id,
                        method_name,
                        "PromptForChoiceMultipleSelection handler failed".to_string(),
                    ),
                }
            }
        };

        if submitter
            .submit(HostResponse {
                call_id,
                scope,
                submission,
            })
            .is_err()
        {
            error!("failed to submit host call response");
            break;
        }
    }
}
