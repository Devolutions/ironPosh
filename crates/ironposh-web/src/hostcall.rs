use futures::StreamExt;
use ironposh_async::HostResponse;
use ironposh_client_core::host::HostCall;
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

/// Macro to handle host calls with automatic JS conversion and error handling
macro_rules! handle_host_call {
    // Main entry point
    ($transport:expr, $handler:expr, $this:expr, $js_params:expr, $method:expr, $($return_type:tt)+) => {{
        let (_, rt) = $transport.into_parts();

        let Ok(res) = call_js_handler(&$handler, &$this, &$js_params, $method).await else {
            continue;
        };

        handle_host_call!(@convert res, rt, $method, $($return_type)+)
    }};

    // Convert String
    (@convert $res:expr, $rt:expr, $method:expr, String) => {{
        let Some(value) = $res.as_string() else {
            error!(method = $method, "expected string return type");
            continue;
        };
        $rt.accept_result(value)
    }};

    // Convert unit/void
    (@convert $res:expr, $rt:expr, $method:expr, ()) => {{
        let _ = $res; // Ensure the promise was awaited
        $rt.accept_result(())
    }};

    // Convert i32
    (@convert $res:expr, $rt:expr, $method:expr, i32) => {{
        let Some(value) = $res.as_f64().map(|v| v as i32) else {
            error!(method = $method, "expected number return type");
            continue;
        };
        $rt.accept_result(value)
    }};

    // Convert bool
    (@convert $res:expr, $rt:expr, $method:expr, bool) => {{
        let value = $res.as_bool().unwrap_or(false);
        $rt.accept_result(value)
    }};

    // Convert uuid::Uuid
    (@convert $res:expr, $rt:expr, $method:expr, uuid::Uuid) => {{
        let Some(uuid_str) = $res.as_string() else {
            error!(method = $method, "expected string for UUID");
            continue;
        };
        let Ok(value) = uuid::Uuid::parse_str(&uuid_str) else {
            error!(method = $method, uuid = %uuid_str, "invalid UUID string");
            continue;
        };
        $rt.accept_result(value)
    }};
}

/// This should definately be handled by JS side, but for now we leave it like this so at least the session's loop is not blocked
pub async fn handle_host_calls(
    mut host_call_rx: futures::channel::mpsc::UnboundedReceiver<HostCall>,
    submitter: ironposh_async::HostSubmitter,
    host_call_handler: js_sys::Function,
) {
    while let Some(host_call) = host_call_rx.next().await {
        let scope = host_call.scope();
        let call_id = host_call.call_id();
        let method_name = host_call.method_name();

        let js_host_call: JsHostCall = (&host_call).into();
        let this = JsValue::NULL;
        let js_params = JsValue::from(js_host_call);
        // let result = host_call_handler.call1(&this, &js_params)?;
        let submission = match host_call {
            HostCall::GetName { transport } => {
                handle_host_call!(
                    transport,
                    host_call_handler,
                    this,
                    js_params,
                    method_name,
                    String
                )
            }
            HostCall::GetVersion { transport } => {
                handle_host_call!(
                    transport,
                    host_call_handler,
                    this,
                    js_params,
                    method_name,
                    String
                )
            }
            HostCall::GetCurrentCulture { transport } => {
                handle_host_call!(
                    transport,
                    host_call_handler,
                    this,
                    js_params,
                    method_name,
                    String
                )
            }
            HostCall::GetCurrentUICulture { transport } => {
                handle_host_call!(
                    transport,
                    host_call_handler,
                    this,
                    js_params,
                    method_name,
                    String
                )
            }
            HostCall::SetShouldExit { transport } => {
                handle_host_call!(
                    transport,
                    host_call_handler,
                    this,
                    js_params,
                    method_name,
                    ()
                )
            }
            HostCall::GetForegroundColor { transport } => {
                handle_host_call!(
                    transport,
                    host_call_handler,
                    this,
                    js_params,
                    method_name,
                    i32
                )
            }
            HostCall::GetBackgroundColor { transport } => {
                handle_host_call!(
                    transport,
                    host_call_handler,
                    this,
                    js_params,
                    method_name,
                    i32
                )
            }
            HostCall::GetKeyAvailable { transport } => {
                handle_host_call!(
                    transport,
                    host_call_handler,
                    this,
                    js_params,
                    method_name,
                    bool
                )
            }
            HostCall::GetInstanceId { transport } => {
                handle_host_call!(
                    transport,
                    host_call_handler,
                    this,
                    js_params,
                    method_name,
                    uuid::Uuid
                )
            }
            HostCall::SetCursorPosition { transport } => {
                handle_host_call!(
                    transport,
                    host_call_handler,
                    this,
                    js_params,
                    method_name,
                    ()
                )
            }
            HostCall::SetBufferContents1 { transport } => {
                handle_host_call!(
                    transport,
                    host_call_handler,
                    this,
                    js_params,
                    method_name,
                    ()
                )
            }
            HostCall::WriteProgress { transport } => {
                handle_host_call!(
                    transport,
                    host_call_handler,
                    this,
                    js_params,
                    method_name,
                    ()
                )
            }
            _ => {
                warn!(method = %host_call.method_name(), "unhandled host call");
                panic!("Unhandled host call: {}", host_call.method_name())
            }
        };

        if submitter
            .submit(HostResponse {
                call_id,
                scope,
                submission,
            })
            .await
            .is_err()
        {
            error!("failed to submit host call response");
            break;
        }
    }
}
