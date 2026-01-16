use futures::StreamExt;
use ironposh_async::HostResponse;
use ironposh_client_core::host::{HostCall, Submission};
use ironposh_psrp::{PipelineHostResponse, PsValue};
use js_sys::Promise;
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
            HostCall::PromptForChoice { transport: _ } => {
                warn!(
                    method = method_name,
                    "PromptForChoice is not implemented; returning exception"
                );
                exception_submission(
                    call_id,
                    method_id,
                    method_name,
                    "PromptForChoice not implemented".to_string(),
                )
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
            HostCall::GetCursorSize { transport: _ } => {
                warn!(
                    method = method_name,
                    "GetCursorSize is not implemented; returning exception"
                );
                exception_submission(
                    call_id,
                    method_id,
                    method_name,
                    "GetCursorSize not implemented".to_string(),
                )
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
                warn!(
                    method = method_name,
                    "GetIsRunspacePushed is not implemented; returning false"
                );
                let ((), rt) = transport.into_parts();
                rt.accept_result(false)
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
                warn!(
                    method = method_name,
                    "ReadLineAsSecureString is not implemented; returning empty bytes"
                );
                let ((), rt) = transport.into_parts();
                rt.accept_result(Vec::new())
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
            HostCall::Prompt { transport: _ } => {
                warn!(
                    method = method_name,
                    "Prompt is not implemented; returning exception"
                );
                exception_submission(
                    call_id,
                    method_id,
                    method_name,
                    "Prompt not implemented".to_string(),
                )
            }
            HostCall::PromptForCredential1 { transport: _ } => {
                warn!(
                    method = method_name,
                    "PromptForCredential1 is not implemented; returning exception"
                );
                exception_submission(
                    call_id,
                    method_id,
                    method_name,
                    "PromptForCredential1 not implemented".to_string(),
                )
            }
            HostCall::PromptForCredential2 { transport: _ } => {
                warn!(
                    method = method_name,
                    "PromptForCredential2 is not implemented; returning exception"
                );
                exception_submission(
                    call_id,
                    method_id,
                    method_name,
                    "PromptForCredential2 not implemented".to_string(),
                )
            }
            HostCall::GetCursorPosition { transport: _ } => {
                warn!(
                    method = method_name,
                    "GetCursorPosition is not implemented; returning exception"
                );
                exception_submission(
                    call_id,
                    method_id,
                    method_name,
                    "GetCursorPosition not implemented".to_string(),
                )
            }
            HostCall::GetWindowPosition { transport: _ } => {
                warn!(
                    method = method_name,
                    "GetWindowPosition is not implemented; returning exception"
                );
                exception_submission(
                    call_id,
                    method_id,
                    method_name,
                    "GetWindowPosition not implemented".to_string(),
                )
            }
            HostCall::GetBufferSize { transport: _ } => {
                warn!(
                    method = method_name,
                    "GetBufferSize is not implemented; returning exception"
                );
                exception_submission(
                    call_id,
                    method_id,
                    method_name,
                    "GetBufferSize not implemented".to_string(),
                )
            }
            HostCall::GetWindowSize { transport: _ } => {
                warn!(
                    method = method_name,
                    "GetWindowSize is not implemented; returning exception"
                );
                exception_submission(
                    call_id,
                    method_id,
                    method_name,
                    "GetWindowSize not implemented".to_string(),
                )
            }
            HostCall::GetMaxWindowSize { transport: _ } => {
                warn!(
                    method = method_name,
                    "GetMaxWindowSize is not implemented; returning exception"
                );
                exception_submission(
                    call_id,
                    method_id,
                    method_name,
                    "GetMaxWindowSize not implemented".to_string(),
                )
            }
            HostCall::GetMaxPhysicalWindowSize { transport: _ } => {
                warn!(
                    method = method_name,
                    "GetMaxPhysicalWindowSize is not implemented; returning exception"
                );
                exception_submission(
                    call_id,
                    method_id,
                    method_name,
                    "GetMaxPhysicalWindowSize not implemented".to_string(),
                )
            }
            HostCall::ReadKey { transport: _ } => {
                warn!(
                    method = method_name,
                    "ReadKey is not implemented; returning exception"
                );
                exception_submission(
                    call_id,
                    method_id,
                    method_name,
                    "ReadKey not implemented".to_string(),
                )
            }
            HostCall::GetBufferContents { transport: _ } => {
                warn!(
                    method = method_name,
                    "GetBufferContents is not implemented; returning exception"
                );
                exception_submission(
                    call_id,
                    method_id,
                    method_name,
                    "GetBufferContents not implemented".to_string(),
                )
            }
            HostCall::GetRunspace { transport: _ } => {
                warn!(
                    method = method_name,
                    "GetRunspace is not implemented; returning exception"
                );
                exception_submission(
                    call_id,
                    method_id,
                    method_name,
                    "GetRunspace not implemented".to_string(),
                )
            }
            HostCall::PromptForChoiceMultipleSelection { transport: _ } => {
                warn!(
                    method = method_name,
                    "PromptForChoiceMultipleSelection is not implemented; returning exception"
                );
                exception_submission(
                    call_id,
                    method_id,
                    method_name,
                    "PromptForChoiceMultipleSelection not implemented".to_string(),
                )
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
