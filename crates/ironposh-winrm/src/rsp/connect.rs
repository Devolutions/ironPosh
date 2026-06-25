use crate::cores::{
    Tag, Text,
    tag_name::{ConnectResponseXml, ConnectXml},
};
use ironposh_macros::{FromXml, SimpleTagValue};

/// Value for the Connect element (MS-WSMV 3.1.4.15).
///
/// Carries the base64 PSRP payload (SESSION_CAPABILITY + CONNECT_RUNSPACEPOOL)
/// in a `connectXml` child element, analogous to `creationXml` on shell create.
#[derive(Debug, Clone, SimpleTagValue, FromXml)]
pub struct ConnectValue<'a> {
    pub connect_xml: Tag<'a, Text<'a>, ConnectXml>,
}

/// Value for the ConnectResponse element (MS-WSMV 3.1.4.15).
///
/// Carries the base64 PSRP payload (SESSION_CAPABILITY + RUNSPACEPOOL_INIT_DATA)
/// in a `connectResponseXml` child element. The payload is optional so callers
/// can surface a descriptive error instead of failing the whole envelope parse.
#[derive(Debug, Clone, SimpleTagValue, FromXml)]
pub struct ConnectResponseValue<'a> {
    pub connect_response_xml: Option<Tag<'a, Text<'a>, ConnectResponseXml>>,
}
