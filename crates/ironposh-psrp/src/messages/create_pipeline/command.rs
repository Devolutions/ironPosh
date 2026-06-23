use super::{CommandParameter, PipelineResultTypes};
use ironposh_macros::{PsDeserialize, PsSerialize};

/// A single pipeline command (MS-PSRP §2.2.3.11).
///
/// The object's `<ToString>` is the command text; `Args` is an ArrayList of
/// CommandParameter; the eight merge-result fields are PipelineResultTypes.
#[derive(Debug, Clone, PartialEq, Eq, typed_builder::TypedBuilder, PsSerialize, PsDeserialize)]
pub struct Command {
    #[builder(setter(into))]
    #[ps(name = "Cmd", to_string)]
    pub cmd: String,
    #[builder(default = false)]
    #[ps(name = "IsScript")]
    pub is_script: bool,
    #[builder(default)]
    #[ps(name = "Args")]
    pub args: Vec<CommandParameter>,
    #[builder(default)]
    #[ps(name = "UseLocalScope", nil_when_none)]
    pub use_local_scope: Option<bool>,
    #[builder(default)]
    #[ps(
        name = "MergeMyResult",
        with = "super::pipeline_result_types::merge_result_conv",
        default
    )]
    pub merge_my_result: PipelineResultTypes,
    #[builder(default)]
    #[ps(
        name = "MergeToResult",
        with = "super::pipeline_result_types::merge_result_conv",
        default
    )]
    pub merge_to_result: PipelineResultTypes,
    #[builder(default)]
    #[ps(
        name = "MergePreviousResults",
        with = "super::pipeline_result_types::merge_result_conv",
        default
    )]
    pub merge_previous_results: PipelineResultTypes,
    #[builder(default)]
    #[ps(
        name = "MergeDebug",
        with = "super::pipeline_result_types::merge_result_conv",
        default
    )]
    pub merge_debug: PipelineResultTypes,
    #[builder(default)]
    #[ps(
        name = "MergeError",
        with = "super::pipeline_result_types::merge_result_conv",
        default
    )]
    pub merge_error: PipelineResultTypes,
    #[builder(default)]
    #[ps(
        name = "MergeInformation",
        with = "super::pipeline_result_types::merge_result_conv",
        default
    )]
    pub merge_information: PipelineResultTypes,
    #[builder(default)]
    #[ps(
        name = "MergeVerbose",
        with = "super::pipeline_result_types::merge_result_conv",
        default
    )]
    pub merge_verbose: PipelineResultTypes,
    #[builder(default)]
    #[ps(
        name = "MergeWarning",
        with = "super::pipeline_result_types::merge_result_conv",
        default
    )]
    pub merge_warning: PipelineResultTypes,
}
