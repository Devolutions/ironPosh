use protocol_macros::{SimpleTagValue, SimpleXmlDeserialize};
use xml::parser::{XmlDeserialize, XmlVisitor};

use crate::{cores::*, rsp::rsp::ShellValue};

#[derive(Debug, Clone, typed_builder::TypedBuilder, SimpleTagValue, SimpleXmlDeserialize)]
pub struct SoapBody<'a> {
    /// WS-Management operations
    #[builder(default, setter(into, strip_option))]
    pub identify: Option<Tag<'a, Empty, Identify>>,
    #[builder(default, setter(into, strip_option))]
    pub get: Option<Tag<'a, Text<'a>, Get>>,
    #[builder(default, setter(into, strip_option))]
    pub put: Option<Tag<'a, Text<'a>, Put>>,
    #[builder(default, setter(into, strip_option))]
    pub create: Option<Tag<'a, Text<'a>, Create>>,
    #[builder(default, setter(into, strip_option))]
    pub delete: Option<Tag<'a, Text<'a>, Delete>>,
    #[builder(default, setter(into, strip_option))]
    pub enumerate: Option<Tag<'a, TagList<'a>, Enumerate>>,
    #[builder(default, setter(into, strip_option))]
    pub pull: Option<Tag<'a, TagList<'a>, Pull>>,
    #[builder(default, setter(into, strip_option))]
    pub release: Option<Tag<'a, TagList<'a>, Release>>,
    #[builder(default, setter(into, strip_option))]
    pub get_status: Option<Tag<'a, TagList<'a>, GetStatus>>,

    /// PowerShell Remoting operations
    #[builder(default, setter(into, strip_option))]
    pub shell: Option<Tag<'a, ShellValue<'a>, Shell>>,
    #[builder(default, setter(into, strip_option))]
    pub command: Option<Tag<'a, TagList<'a>, Command>>,
    #[builder(default, setter(into, strip_option))]
    pub receive: Option<Tag<'a, TagList<'a>, Receive>>,
    #[builder(default, setter(into, strip_option))]
    pub send: Option<Tag<'a, TagList<'a>, Send>>,
    #[builder(default, setter(into, strip_option))]
    pub signal: Option<Tag<'a, TagList<'a>, Signal>>,
}
