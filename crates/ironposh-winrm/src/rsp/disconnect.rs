use crate::cores::{
    Tag, Time,
    tag_name::{IdleTimeOut, TagName},
};
use ironposh_macros::{SimpleTagValue, SimpleXmlDeserialize};

/// Value for the Disconnect element (MS-WSMV 3.1.4.13).
/// Optionally carries an IdleTimeOut serialized as `PT{seconds}S`.
#[derive(Debug, Clone, typed_builder::TypedBuilder, SimpleTagValue, SimpleXmlDeserialize)]
pub struct DisconnectValue<'a> {
    #[builder(default, setter(strip_option, into))]
    pub idle_time_out: Option<Tag<'a, Time, IdleTimeOut>>,
}
