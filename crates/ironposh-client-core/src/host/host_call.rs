use super::{
    HostError,
    conversions::RemoteHostMethodId,
    methods,
    traits::{FromParams, Method, sealed},
    transports::Transport,
    types::HostCallScope,
};
use ironposh_psrp::{PipelineHostCall, PsValue};

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
                                transport: Transport::new(scope, phc.call_id, params)
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

    // UI methods (11-26)
    ReadLine = ReadLine: () -> String,
    ReadLineAsSecureString = ReadLineAsSecureString: () -> Vec<u8>,
    Write1 = Write1: (String) -> (),
    Write2 = Write2: (i32, i32, String) -> (),
    WriteLine1 = WriteLine1: () -> (),
    WriteLine2 = WriteLine2: (String) -> (),
    WriteLine3 = WriteLine3: (i32, i32, String) -> (),
    WriteErrorLine = WriteErrorLine: (String) -> (),
    WriteDebugLine = WriteDebugLine: (String) -> (),
    WriteProgress = WriteProgress: (i64, methods::ProgressRecord) -> (),
    WriteVerboseLine = WriteVerboseLine: (String) -> (),
    WriteWarningLine = WriteWarningLine: (String) -> (),
    Prompt = Prompt: (String, String, Vec<methods::FieldDescription>) -> std::collections::HashMap<String, PsValue>,
    PromptForCredential1 = PromptForCredential1: (String, String, String, String) -> methods::PSCredential,
    PromptForCredential2 = PromptForCredential2: (String, String, String, String, i32, i32) -> methods::PSCredential,
    PromptForChoice = PromptForChoice: (String, String, Vec<methods::ChoiceDescription>, i32) -> i32,

    // RawUI methods (27-51)
    GetForegroundColor = GetForegroundColor: () -> i32,
    SetForegroundColor = SetForegroundColor: (i32) -> (),
    GetBackgroundColor = GetBackgroundColor: () -> i32,
    SetBackgroundColor = SetBackgroundColor: (i32) -> (),
    GetCursorPosition = GetCursorPosition: () -> methods::Coordinates,
    SetCursorPosition = SetCursorPosition: (methods::Coordinates) -> (),
    GetWindowPosition = GetWindowPosition: () -> methods::Coordinates,
    SetWindowPosition = SetWindowPosition: (methods::Coordinates) -> (),
    GetCursorSize = GetCursorSize: () -> i32,
    SetCursorSize = SetCursorSize: (i32) -> (),
    GetBufferSize = GetBufferSize: () -> methods::Size,
    SetBufferSize = SetBufferSize: (methods::Size) -> (),
    GetWindowSize = GetWindowSize: () -> methods::Size,
    SetWindowSize = SetWindowSize: (methods::Size) -> (),
    GetWindowTitle = GetWindowTitle: () -> String,
    SetWindowTitle = SetWindowTitle: (String) -> (),
    GetMaxWindowSize = GetMaxWindowSize: () -> methods::Size,
    GetMaxPhysicalWindowSize = GetMaxPhysicalWindowSize: () -> methods::Size,
    GetKeyAvailable = GetKeyAvailable: () -> bool,
    ReadKey = ReadKey: (i32) -> methods::KeyInfo,
    FlushInputBuffer = FlushInputBuffer: () -> (),
    SetBufferContents1 = SetBufferContents1: (methods::Coordinates, Vec<Vec<methods::BufferCell>>) -> (),
    SetBufferContents2 = SetBufferContents2: (methods::Rectangle, methods::BufferCell) -> (),
    GetBufferContents = GetBufferContents: (methods::Rectangle) -> Vec<Vec<methods::BufferCell>>,
    ScrollBufferContents = ScrollBufferContents: (methods::Rectangle, methods::Coordinates, methods::Rectangle, methods::BufferCell) -> (),

    // Interactive session methods (52-56)
    PushRunspace = PushRunspace: (PsValue) -> (),
    PopRunspace = PopRunspace: () -> (),
    GetIsRunspacePushed = GetIsRunspacePushed: () -> bool,
    GetRunspace = GetRunspace: () -> PsValue,
    PromptForChoiceMultipleSelection = PromptForChoiceMultipleSelection: (String, String, Vec<methods::ChoiceDescription>, Vec<i32>) -> Vec<i32>,
}
