use protocol_macros::{SimpleTagValue, SimpleXmlDeserialize};

use crate::cores::{tag_name::TagName, DesiredStream, Tag, Text};

#[derive(Debug, Clone, typed_builder::TypedBuilder, SimpleTagValue, SimpleXmlDeserialize)]
pub struct ReceiveValue<'a> {
    pub desired_stream: Tag<'a, Text<'a>, DesiredStream>,
}
