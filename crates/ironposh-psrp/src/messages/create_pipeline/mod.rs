mod command;
mod command_parameter;
mod pipeline_result_types;
mod powershell_pipeline;
mod remote_stream_options;
#[cfg(test)]
mod test;

pub use command::Command;
pub use command_parameter::CommandParameter;
pub use pipeline_result_types::PipelineResultTypes;
pub use powershell_pipeline::PowerShellPipeline;
pub use remote_stream_options::RemoteStreamOptions;

use super::init_runspace_pool::{ApartmentState, HostInfo};
use ironposh_macros::{PsDeserialize, PsSerialize};

/// CREATE_PIPELINE (MS-PSRP §2.2.2.10): client → server. Fully macro-derived;
/// each field maps to its `<MS>` property and nested objects/enums supply their
/// own conversions.
#[derive(Debug, Clone, PartialEq, Eq, typed_builder::TypedBuilder, PsSerialize, PsDeserialize)]
#[ps(message_type = CreatePipeline, type_names("System.Object"))]
pub struct CreatePipeline {
    #[builder(default = true)]
    #[ps(name = "NoInput")]
    pub no_input: bool,
    #[builder(default = ApartmentState::Unknown)]
    #[ps(name = "ApartmentState")]
    pub apartment_state: ApartmentState,
    #[builder(default = RemoteStreamOptions::None)]
    #[ps(name = "RemoteStreamOptions")]
    pub remote_stream_options: RemoteStreamOptions,
    #[builder(default = false)]
    #[ps(name = "AddToHistory")]
    pub add_to_history: bool,
    #[ps(name = "HostInfo")]
    pub host_info: HostInfo,
    #[ps(name = "PowerShell")]
    pub pipeline: PowerShellPipeline,
    #[builder(default = false)]
    #[ps(name = "IsNested")]
    pub is_nested: bool,
}
