pub mod apartment_state;
pub mod application_arguments;
pub mod application_private_data;
pub mod host_default_data;
pub mod host_info;
pub mod ps_thread_options;

pub use apartment_state::ApartmentState;
pub use application_arguments::{ApplicationArguments, PSVersionTable};
pub use application_private_data::ApplicationPrivateData;
pub use host_default_data::{Coordinates, HostDefaultData, Size};
pub use host_info::HostInfo;
pub use ps_thread_options::PSThreadOptions;

use ironposh_macros::PsSerialize;

/// INIT_RUNSPACEPOOL (MS-PSRP §2.2.2.3): client → server. Fully macro-derived;
/// `application_arguments` collapses to `Nil` when empty via `app_args_conv`.
#[derive(Debug, Clone, PartialEq, Eq, PsSerialize)]
#[ps(message_type = InitRunspacepool)]
pub struct InitRunspacePool {
    #[ps(name = "MinRunspaces")]
    pub min_runspaces: i32,
    #[ps(name = "MaxRunspaces")]
    pub max_runspaces: i32,
    #[ps(name = "PSThreadOptions")]
    pub thread_options: PSThreadOptions,
    #[ps(name = "ApartmentState")]
    pub apartment_state: ApartmentState,
    #[ps(name = "HostInfo")]
    pub host_info: HostInfo,
    #[ps(name = "ApplicationArguments", with = "app_args_conv")]
    pub application_arguments: ApplicationArguments,
}

/// `#[ps(with)]`: emit `ApplicationArguments` as `Nil` when empty, else the
/// derived PSPrimitiveDictionary object. INIT_RUNSPACEPOOL is client → server
/// only, so just the serialize half is needed.
mod app_args_conv {
    use super::ApplicationArguments;
    use crate::ps_value::{PsPrimitiveValue, PsValue, ToPsValue};

    pub fn to_ps_value(args: &ApplicationArguments) -> PsValue {
        if args.is_empty() {
            PsValue::Primitive(PsPrimitiveValue::Nil)
        } else {
            args.to_ps_value()
        }
    }
}
