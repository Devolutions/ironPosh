pub mod body;
pub mod header;
pub use header::*;

pub const WSMAN_NAMESPACE: &str = "http://schemas.microsoft.com/wbem/wsman/1/wsman.xsd";
pub const WSMAN_NAMESPACE_ALIAS: &str = "w";

#[macro_export]
macro_rules! wsman_ns {
    () => {
        xml::builder::Namespace::new($crate::ws_management::WSMAN_NAMESPACE)
    };
}
