mod conversions;
mod error;
mod methods;
mod types;

pub use error::*;
pub use types::*;

// Export spec-compliant utilities
pub use conversions::{RemoteHostMethodId, should_send_host_response};

use ironposh_psrp::{PipelineHostCall, PipelineHostResponse, PsValue};
use core::marker::PhantomData;

//========================================================================================
// NEW TYPESAFE HOST CALL SYSTEM
//========================================================================================

/// Sealed trait for compile-time method type safety
pub trait Method: sealed::Sealed {
    const ID: RemoteHostMethodId;
    type Params;
    type Return; // () = void
}

mod sealed { 
    pub trait Sealed {} 
}

/// Parameter extraction from pipeline values
pub trait FromParams: Sized { 
    fn from_params(args: &[PsValue]) -> Result<Self, HostError>; 
}

/// Return value encoding to pipeline values
pub trait ToPs { 
    fn to_ps(v: Self) -> Option<PsValue>; 
}

// Implement basic parameter/return conversions
impl FromParams for () { 
    fn from_params(a: &[PsValue]) -> Result<Self, HostError> { 
        if a.is_empty() { Ok(()) } else { Err(HostError::InvalidParameters) } 
    } 
}

impl FromParams for String { 
    fn from_params(a: &[PsValue]) -> Result<Self, HostError> { 
        a.get(0)
            .and_then(|v| v.as_string())
            .ok_or(HostError::InvalidParameters) 
    } 
}

impl FromParams for i32 {
    fn from_params(a: &[PsValue]) -> Result<Self, HostError> {
        a.get(0)
            .and_then(|v| v.as_i32())
            .ok_or(HostError::InvalidParameters)
    }
}

impl FromParams for (i32,) {
    fn from_params(a: &[PsValue]) -> Result<Self, HostError> {
        let param = a.get(0).and_then(|v| v.as_i32()).ok_or(HostError::InvalidParameters)?;
        Ok((param,))
    }
}

impl FromParams for (String,) {
    fn from_params(a: &[PsValue]) -> Result<Self, HostError> {
        let param = a.get(0).and_then(|v| v.as_string()).ok_or(HostError::InvalidParameters)?;
        Ok((param,))
    }
}

// TODO: Add more parameter converters as needed
impl FromParams for methods::Coordinates {
    fn from_params(_a: &[PsValue]) -> Result<Self, HostError> {
        // For now, just return a default - this needs proper coordinate parsing from PsValue
        Ok(methods::Coordinates { x: 0, y: 0 })
    }
}

impl ToPs for String { 
    fn to_ps(v: String) -> Option<PsValue> { 
        Some(PsValue::from(v)) 
    } 
}

impl ToPs for i32 {
    fn to_ps(v: i32) -> Option<PsValue> {
        Some(PsValue::from(v))
    }
}

impl ToPs for bool {
    fn to_ps(v: bool) -> Option<PsValue> {
        Some(PsValue::from(v))
    }
}

impl ToPs for uuid::Uuid {
    fn to_ps(v: uuid::Uuid) -> Option<PsValue> {
        Some(PsValue::from(v.to_string()))
    }
}

impl ToPs for Vec<u8> {
    fn to_ps(v: Vec<u8>) -> Option<PsValue> {
        Some(PsValue::from(v))
    }
}

// TODO: Add more return converters as needed
impl ToPs for methods::Coordinates {
    fn to_ps(_v: methods::Coordinates) -> Option<PsValue> {
        // For now, just return None - this needs proper coordinate to PsValue conversion
        None
    }
}

impl ToPs for methods::Size {
    fn to_ps(_v: methods::Size) -> Option<PsValue> {
        // For now, just return None - this needs proper size to PsValue conversion
        None
    }
}

impl ToPs for () { 
    fn to_ps(_: ()) -> Option<PsValue> { 
        None  // Void methods don't return values
    } 
}

/// Transport wraps method parameters and provides typed result submission
#[derive(Debug)]
pub struct Transport<M: Method> {
    pub scope: HostCallScope,
    pub call_id: i64,
    pub params: M::Params,
    _m: PhantomData<M>,
}

impl<M: Method> Transport<M> {
    pub fn into_parts(self) -> (M::Params, ResultTransport<M>) {
        (self.params, ResultTransport { 
            scope: self.scope, 
            call_id: self.call_id, 
            _m: PhantomData
        })
    }
}

/// Result transport handles typed return values and creates pipeline responses
pub struct ResultTransport<M: Method> {
    scope: HostCallScope,
    call_id: i64,
    _m: PhantomData<M>,
}

/// What gets passed back to the session
#[derive(Debug)]
pub enum Submission { 
    Send(PipelineHostResponse), 
    NoSend 
}

impl<M: Method> ResultTransport<M> {
    /// Accept a result - automatically determines if response should be sent based on method
    pub fn accept_result(self, v: M::Return) -> Submission
    where 
        M::Return: ToPs,
    {
        if should_send_host_response(M::ID) {
            Submission::Send(PipelineHostResponse {
                call_id: self.call_id,
                method_id: M::ID as i32,
                method_name: format!("{:?}", M::ID),
                method_result: <M::Return as ToPs>::to_ps(v),
                method_exception: None,
            })
        } else {
            Submission::NoSend
        }
    }
}

//========================================================================================
// METHOD DEFINITIONS - All MS-PSRP spec methods via macro
//========================================================================================

macro_rules! define_host_methods {
    ($(
        $method_name:ident = $method_id:ident : ($($param:ty),*) -> $return:ty
    ),* $(,)?) => {
        // Define method structs
        $(
            #[derive(Debug)]
            pub struct $method_name;
            impl sealed::Sealed for $method_name {}
            impl Method for $method_name {
                const ID: RemoteHostMethodId = RemoteHostMethodId::$method_id;
                type Params = ($($param,)*);
                type Return = $return;
            }
        )*

        /// The single enum for all host method calls - compile-time typed
        #[derive(Debug)]
        pub enum HostCall {
            $(
                $method_name { transport: Transport<$method_name> },
            )*
        }

        impl HostCall {
            /// Convert from pipeline host call to typesafe host call
            pub fn try_from_pipeline(scope: HostCallScope, phc: PipelineHostCall) -> Result<Self, HostError> {
                let id = RemoteHostMethodId::try_from(phc.method_id)?;
                
                match id {
                    $(
                        RemoteHostMethodId::$method_id => {
                            let params: <$method_name as Method>::Params = FromParams::from_params(&phc.parameters)?;
                            Ok(HostCall::$method_name { 
                                transport: Transport { 
                                    scope, 
                                    call_id: phc.call_id, 
                                    params, 
                                    _m: PhantomData
                                } 
                            })
                        }
                    )*
                    _ => Err(HostError::NotImplemented),
                }
            }

            /// Get the call ID for this host call
            pub fn call_id(&self) -> i64 {
                match self {
                    $(
                        HostCall::$method_name { transport } => transport.call_id,
                    )*
                }
            }

            /// Get the method name for this host call
            pub fn method_name(&self) -> &'static str {
                match self {
                    $(
                        HostCall::$method_name { .. } => stringify!($method_name),
                    )*
                }
            }

            /// Get the scope for this host call
            pub fn scope(&self) -> HostCallScope {
                match self {
                    $(
                        HostCall::$method_name { transport } => transport.scope.clone(),
                    )*
                }
            }

            /// Get the method ID for this host call
            pub fn method_id(&self) -> i32 {
                match self {
                    $(
                        HostCall::$method_name { .. } => RemoteHostMethodId::$method_id as i32,
                    )*
                }
            }
        }
    };
}

// Define all methods following MS-PSRP spec
define_host_methods! {
    // Host methods (1-10)
    GetName = GetName: () -> String,
    GetVersion = GetVersion: () -> String,
    GetInstanceId = GetInstanceId: () -> uuid::Uuid,
    GetCurrentCulture = GetCurrentCulture: () -> String,
    GetCurrentUICulture = GetCurrentUICulture: () -> String,
    SetShouldExit = SetShouldExit: (i32) -> (),
    EnterNestedPrompt = EnterNestedPrompt: () -> (),
    ExitNestedPrompt = ExitNestedPrompt: () -> (),
    NotifyBeginApplication = NotifyBeginApplication: () -> (),
    NotifyEndApplication = NotifyEndApplication: () -> (),

    // UI methods (11-26) - starting with simple ones
    ReadLine = ReadLine: () -> String,
    ReadLineAsSecureString = ReadLineAsSecureString: () -> Vec<u8>,
    Write1 = Write1: (String) -> (),
    WriteLine1 = WriteLine1: () -> (),
    WriteLine2 = WriteLine2: (String) -> (),
    WriteErrorLine = WriteErrorLine: (String) -> (),
    WriteDebugLine = WriteDebugLine: (String) -> (),
    WriteVerboseLine = WriteVerboseLine: (String) -> (),
    WriteWarningLine = WriteWarningLine: (String) -> (),

    // RawUI methods (27-51) - starting with simple ones
    GetForegroundColor = GetForegroundColor: () -> i32,
    SetForegroundColor = SetForegroundColor: (i32) -> (),
    GetBackgroundColor = GetBackgroundColor: () -> i32,
    SetBackgroundColor = SetBackgroundColor: (i32) -> (),
    GetCursorSize = GetCursorSize: () -> i32,
    SetCursorSize = SetCursorSize: (i32) -> (),
    GetWindowTitle = GetWindowTitle: () -> String,
    SetWindowTitle = SetWindowTitle: (String) -> (),
    GetKeyAvailable = GetKeyAvailable: () -> bool,
    FlushInputBuffer = FlushInputBuffer: () -> (),

    // Interactive session methods (52-55) - simple ones first
    PopRunspace = PopRunspace: () -> (),
    GetIsRunspacePushed = GetIsRunspacePushed: () -> bool,
}

//========================================================================================
// MAIN HOST CALL ENUM
//========================================================================================


