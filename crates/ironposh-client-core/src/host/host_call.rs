use super::{
    HostError, methods,
    traits::{FromParams, Method, sealed},
    transports::Transport,
    types::HostCallScope,
};
use ironposh_psrp::{PipelineHostCall, PsValue};

macro_rules! define_host_methods {
    ($(
        $method_id:literal . $method_name:ident : ($($param:ty),*) -> $return:ty, send_back = $send_back:literal
    ),* $(,)?) => {
        // Define method structs
        $(
            #[derive(Debug)]
            pub struct $method_name;
            impl sealed::Sealed for $method_name {}
            impl Method for $method_name {
                const ID: i32 = $method_id;
                const NAME: &'static str = stringify!($method_name);
                type Params = ($($param,)*);
                type Return = $return;

                fn should_send_response() -> bool {
                    $send_back
                }
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
                match phc.method_id {
                    $(
                        $method_id => {
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
                        HostCall::$method_name { .. } => <$method_name as Method>::ID,
                    )*
                }
            }

            /// Check if this method should send a response
            pub fn should_send_response(&self) -> bool {
                match self {
                    $(
                        HostCall::$method_name { .. } => <$method_name as Method>::should_send_response(),
                    )*
                }
            }
        }
    };
}

// Define all methods following MS-PSRP spec
define_host_methods! {
    // Host methods (1-10)
    1.GetName: () -> String, send_back = true,
    2.GetVersion: () -> String, send_back = true,
    3.GetInstanceId: () -> uuid::Uuid, send_back = true,
    4.GetCurrentCulture: () -> String, send_back = true,
    5.GetCurrentUICulture: () -> String, send_back = true,
    6.SetShouldExit: (i32) -> (), send_back = false,
    7.EnterNestedPrompt: () -> (), send_back = false,
    8.ExitNestedPrompt: () -> (), send_back = false,
    9.NotifyBeginApplication: () -> (), send_back = false,
    10.NotifyEndApplication: () -> (), send_back = false,

    // UI methods (11-26)
    11.ReadLine: () -> String, send_back = true,
    12.ReadLineAsSecureString: () -> Vec<u8>, send_back = true,
    13.Write1: (String) -> (), send_back = false,
    14.Write2: (i32, i32, String) -> (), send_back = false,
    15.WriteLine1: () -> (), send_back = false,
    16.WriteLine2: (String) -> (), send_back = false,
    17.WriteLine3: (i32, i32, String) -> (), send_back = false,
    18.WriteErrorLine: (String) -> (), send_back = false,
    19.WriteDebugLine: (String) -> (), send_back = false,
    20.WriteProgress: (i64, methods::ProgressRecord) -> (), send_back = false,
    21.WriteVerboseLine: (String) -> (), send_back = false,
    22.WriteWarningLine: (String) -> (), send_back = false,
    23.Prompt: (String, String, Vec<methods::FieldDescription>) -> std::collections::HashMap<String, PsValue>, send_back = true,
    24.PromptForCredential1: (String, String, String, String) -> methods::PSCredential, send_back = true,
    25.PromptForCredential2: (String, String, String, String, i32, i32) -> methods::PSCredential, send_back = true,
    26.PromptForChoice: (String, String, Vec<methods::ChoiceDescription>, i32) -> i32, send_back = true,

    // RawUI methods (27-51)
    27.GetForegroundColor: () -> i32, send_back = true,
    28.SetForegroundColor: (i32) -> (), send_back = false,
    29.GetBackgroundColor: () -> i32, send_back = true,
    30.SetBackgroundColor: (i32) -> (), send_back = false,
    31.GetCursorPosition: () -> methods::Coordinates, send_back = true,
    32.SetCursorPosition: (methods::Coordinates) -> (), send_back = false,
    33.GetWindowPosition: () -> methods::Coordinates, send_back = true,
    34.SetWindowPosition: (methods::Coordinates) -> (), send_back = false,
    35.GetCursorSize: () -> i32, send_back = true,
    36.SetCursorSize: (i32) -> (), send_back = false,
    37.GetBufferSize: () -> methods::Size, send_back = true,
    38.SetBufferSize: (methods::Size) -> (), send_back = false,
    39.GetWindowSize: () -> methods::Size, send_back = true,
    40.SetWindowSize: (methods::Size) -> (), send_back = false,
    41.GetWindowTitle: () -> String, send_back = true,
    42.SetWindowTitle: (String) -> (), send_back = false,
    43.GetMaxWindowSize: () -> methods::Size, send_back = true,
    44.GetMaxPhysicalWindowSize: () -> methods::Size, send_back = true,
    45.GetKeyAvailable: () -> bool, send_back = true,
    46.ReadKey: (i32) -> methods::KeyInfo, send_back = true,
    47.FlushInputBuffer: () -> (), send_back = false,
    48.SetBufferContents1: (methods::Coordinates, Vec<Vec<methods::BufferCell>>) -> (), send_back = false,
    49.SetBufferContents2: (methods::Rectangle, methods::BufferCell) -> (), send_back = false,
    50.GetBufferContents: (methods::Rectangle) -> Vec<Vec<methods::BufferCell>>, send_back = true,
    51.ScrollBufferContents: (methods::Rectangle, methods::Coordinates, methods::Rectangle, methods::BufferCell) -> (), send_back = false,

    // Interactive session methods (52-56)
    52.PushRunspace: (PsValue) -> (), send_back = false,
    53.PopRunspace: () -> (), send_back = false,
    54.GetIsRunspacePushed: () -> bool, send_back = true,
    55.GetRunspace: () -> PsValue, send_back = true,
    56.PromptForChoiceMultipleSelection: (String, String, Vec<methods::ChoiceDescription>, Vec<i32>) -> Vec<i32>, send_back = true,
}
