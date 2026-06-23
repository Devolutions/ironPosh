use crate::RemoteHostMethodId;
use crate::ps_value::PsValue;
use ironposh_macros::{PsDeserialize, PsSerialize};

/// RUNSPACEPOOL_HOST_RESPONSE (MS-PSRP §2.2.2.16): client → server response to a
/// runspace-pool host call. Same shape as PIPELINE_HOST_RESPONSE.
#[derive(Debug, Clone, PartialEq, Eq, typed_builder::TypedBuilder, PsSerialize, PsDeserialize)]
#[ps(message_type = RunspacepoolHostResponse)]
pub struct RunspacePoolHostResponse {
    #[ps(name = "ci")]
    pub call_id: i64,
    #[ps(name = "mi")]
    pub method: RemoteHostMethodId,
    #[builder(default, setter(strip_option(fallback_suffix = "_opt")))]
    #[ps(name = "mr")]
    pub method_result: Option<PsValue>,
    #[builder(default, setter(strip_option(fallback_suffix = "_opt")))]
    #[ps(name = "me")]
    pub method_exception: Option<PsValue>,
}
