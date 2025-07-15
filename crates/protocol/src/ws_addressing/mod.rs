use xml::{XmlError, parser::Node};

use crate::{
    define_tagname, must_be_text, push_element,
    traits::{MustUnderstand, Tag, Tag1, TagValue},
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
    pub action: Option<Tag1<'a, &'a str, Action, MustUnderstand>>,
    #[builder(default, setter(strip_option, into))]
    pub to: Option<Tag<'a, &'a str, To>>,
    #[builder(default, setter(strip_option, into))]
    pub message_id: Option<Tag<'a, &'a str, MessageID>>,
    #[builder(default, setter(strip_option, into))]
    pub relates_to: Option<Tag<'a, &'a str, RelatesTo>>,
    #[builder(default, setter(strip_option, into))]
    pub reply_to: Option<Tag<'a, Tag1<'a, &'a str, Address, MustUnderstand>, ReplyTo>>,
    #[builder(default, setter(strip_option, into))]
    pub fault_to: Option<Tag<'a, &'a str, FaultTo>>,
    #[builder(default, setter(strip_option, into))]
    pub from: Option<Tag<'a, &'a str, From>>,
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

impl<'a> TryFrom<Vec<Node<'a, 'a>>> for WsAddressingHeaders<'a> {
    type Error = xml::XmlError<'a>;

    fn try_from(value: Vec<Node<'a, 'a>>) -> Result<Self, Self::Error> {
        let mut action = None;
        let mut to = None;
        let mut message_id = None;
        let mut relates_to = None;
        let mut reply_to = None;
        let mut fault_to = None;
        let mut from = None;

        for node in value {
            match node.tag_name().name() {
                "Action" => {
                    let value = {
                        let child = node
                            .first_child()
                            .ok_or(XmlError::GenericError("expecting node".into()))?;
                        must_be_text!(child);
                        let text = child.text().expect("must be text");
                        text
                    };
                    action = Some(value.trim());
                }
                "To" => {
                    let value = {
                        let child = node
                            .first_child()
                            .ok_or(XmlError::GenericError("expecting node".into()))?;
                        must_be_text!(child);
                        let text = child.text().expect("must be text");
                        text
                    };
                    to = Some(value.trim());
                }
                "MessageID" => {
                    let value = {
                        let child = node
                            .first_child()
                            .ok_or(XmlError::GenericError("expecting node".into()))?;
                        must_be_text!(child);
                        let text = child.text().expect("must be text");
                        text
                    };
                    message_id = Some(value.trim());
                }
                "RelatesTo" => {
                    let value = {
                        let child = node
                            .first_child()
                            .ok_or(XmlError::GenericError("expecting node".into()))?;
                        must_be_text!(child);
                        let text = child.text().expect("must be text");
                        text
                    };
                    relates_to = Some(value.trim());
                }
                "ReplyTo" => {
                    let value = {
                        let child = node
                            .first_child()
                            .ok_or(XmlError::GenericError("expecting node".into()))?;
                        must_be_text!(child);
                        let text = child.text().expect("must be text");
                        text
                    };
                    reply_to = Some(value.trim());
                }
                "FaultTo" => {
                    let value = {
                        let child = node
                            .first_child()
                            .ok_or(XmlError::GenericError("expecting node".into()))?;
                        must_be_text!(child);
                        let text = child.text().expect("must be text");
                        text
                    };
                    fault_to = Some(value.trim());
                }
                "From" => {
                    let value = {
                        let child = node
                            .first_child()
                            .ok_or(XmlError::GenericError("expecting node".into()))?;
                        must_be_text!(child);
                        let text = child.text().expect("must be text");
                        text
                    };
                    from = Some(value.trim());
                }
                tag_name => return Err(xml::XmlError::UnexpectedTag(tag_name.into())),
            }
        }

        Ok(WsAddressingHeaders {
            action: action.map(|action| Action::new_tag1(action, MustUnderstand { value: false })),
            to: to.map(|t| To::new_tag(t)),
            message_id: message_id.map(|m| MessageID::new_tag(m)),
            relates_to: relates_to.map(|r| RelatesTo::new_tag(r)),
            // reply_to: reply_to.map(|r| ReplyTo::new_tag(r)),
            // TODO: Fix this
            reply_to: None,
            fault_to: fault_to.map(|r| FaultTo::new_tag(r)),
            from: from.map(|r| From::new_tag(r)),
        })
    }
}

impl<'a> crate::soap::SoapHeaders<'a> for WsAddressingHeaders<'a> {
    const NAMESPACE: &'static str = WSA_NAMESPACE;
}
