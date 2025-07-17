
use crate::{
    define_tagname, push_element,
    traits::{MustUnderstand, Tag, Tag1, tag_value::Text},
};

pub const WSA_NAMESPACE: &str = "http://schemas.xmlsoap.org/ws/2004/08/addressing";
pub const WSA_NAMESPACE_ALIAS: &str = "a";

pub fn headers_builder<'a>() -> WsAddressingHeadersBuilder<'a> {
    WsAddressingHeaders::builder()
}

define_tagname!(Action, Some(WSA_NAMESPACE));
define_tagname!(To, Some(WSA_NAMESPACE));
define_tagname!(MessageID, Some(WSA_NAMESPACE));
define_tagname!(RelatesTo, Some(WSA_NAMESPACE));
define_tagname!(ReplyTo, Some(WSA_NAMESPACE));
define_tagname!(FaultTo, Some(WSA_NAMESPACE));
define_tagname!(From, Some(WSA_NAMESPACE));
define_tagname!(Address, Some(WSA_NAMESPACE));

#[derive(typed_builder::TypedBuilder, Debug, Clone)]
pub struct WsAddressingHeaders<'a> {
    #[builder(default, setter(strip_option, into))]
    pub action: Option<Tag1<'a, Text<'a>, Action, MustUnderstand>>,
    #[builder(default, setter(strip_option, into))]
    pub to: Option<Tag<'a, Text<'a>, To>>,
    #[builder(default, setter(strip_option, into))]
    pub message_id: Option<Tag<'a, Text<'a>, MessageID>>,
    #[builder(default, setter(strip_option, into))]
    pub relates_to: Option<Tag<'a, Text<'a>, RelatesTo>>,
    #[builder(default, setter(strip_option, into))]
    pub reply_to: Option<Tag<'a, Tag1<'a, Text<'a>, Address, MustUnderstand>, ReplyTo>>,
    #[builder(default, setter(strip_option, into))]
    pub fault_to: Option<Tag<'a, Text<'a>, FaultTo>>,
    #[builder(default, setter(strip_option, into))]
    pub from: Option<Tag<'a, Text<'a>, From>>,
}

impl<'a> IntoIterator for WsAddressingHeaders<'a> {
    type Item = xml::builder::Element<'a>;

    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        let WsAddressingHeaders {
            action,
            to,
            message_id,
            relates_to,
            reply_to,
            fault_to,
            from,
        } = self;

        let mut tags = Vec::new();

        push_element!(
            tags,
            [action, to, message_id, relates_to, reply_to, fault_to, from]
        );

        tags.into_iter()
    }
}

impl<'a> crate::soap::SoapHeaders<'a> for WsAddressingHeaders<'a> {
    const NAMESPACE: &'static str = WSA_NAMESPACE;
}
