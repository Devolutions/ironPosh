use crate::RemoteHostMethodId;
use crate::ps_value::PsValue;
use ironposh_macros::{PsDeserialize, PsSerialize};

/// RUNSPACEPOOL_HOST_CALL (MS-PSRP §2.2.2.15): server → client request to run a
/// host method against the RunspacePool's host.
///
/// `ci` = call id, `mi` = the host method (a `RemoteHostMethodId` enum object),
/// `mp` = the method parameters (an `ArrayList`). The params stay `Vec<PsValue>`
/// — their types are method-specific and resolved by the typed host-call layer.
#[derive(Debug, Clone, PartialEq, Eq, typed_builder::TypedBuilder, PsSerialize, PsDeserialize)]
#[ps(message_type = RunspacepoolHostCall)]
pub struct RunspacePoolHostCall {
    #[ps(name = "ci")]
    pub call_id: i64,
    #[ps(name = "mi")]
    pub method: RemoteHostMethodId,
    #[builder(default)]
    #[ps(name = "mp")]
    pub parameters: Vec<PsValue>,
}
