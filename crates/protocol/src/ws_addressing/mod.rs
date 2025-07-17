use crate::{
    push_elements,
    traits::{Tag, TagList, tag_name::*, tag_value::Text},
};

pub fn headers_builder<'a>() -> WsAddressingHeadersBuilder<'a> {
    WsAddressingHeaders::builder()
}

#[derive(typed_builder::TypedBuilder, Debug, Clone)]
pub struct WsAddressingHeaders<'a> {
    #[builder(default, setter(strip_option, into))]
    pub action: Option<Tag<'a, Text<'a>, Action>>,
    #[builder(default, setter(strip_option, into))]
    pub to: Option<Tag<'a, Text<'a>, To>>,
    #[builder(default, setter(strip_option, into))]
    pub message_id: Option<Tag<'a, Text<'a>, MessageID>>,
    #[builder(default, setter(strip_option, into))]
    pub relates_to: Option<Tag<'a, Text<'a>, RelatesTo>>,
    #[builder(default, setter(strip_option, into))]
    pub reply_to: Option<Tag<'a, TagList<'a>, ReplyTo>>,
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

        push_elements!(
            tags,
            [action, to, message_id, relates_to, reply_to, fault_to, from]
        );

        tags.into_iter()
    }
}
