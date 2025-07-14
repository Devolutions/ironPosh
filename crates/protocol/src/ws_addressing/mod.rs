use xml::{XmlError, parser::Node};

use crate::{
    define_tagname, must_be_text,
    traits::{MustUnderstand, Tag, Tag1},
};

pub const WSA_NAMESPACE: &str = "http://schemas.xmlsoap.org/ws/2004/08/addressing";
pub const WSA_NAMESPACE_ALIAS: &str = "a";

macro_rules! wsa_ns {
    () => {
        xml::builder::Namespace::new(WSA_NAMESPACE)
    };
}

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

#[derive(typed_builder::TypedBuilder, Debug, Clone)]
pub struct WsAddressingHeaders<'a> {
    #[builder(setter(into))]
    pub action: Tag1<'a, &'a str, Action, MustUnderstand>,
    #[builder(setter(into))]
    pub to: Tag<'a, &'a str, To>,
    #[builder(setter(into))]
    pub message_id: Tag<'a, &'a str, MessageID>,

    #[builder(default, setter(into))]
    pub relates_to: Option<Tag<'a, &'a str, RelatesTo>>,
    #[builder(default, setter(into))]
    pub reply_to: Option<Tag<'a, &'a str, ReplyTo>>,
    #[builder(default, setter(into))]
    pub fault_to: Option<Tag<'a, &'a str, FaultTo>>,
    #[builder(default, setter(into))]
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

        let action = action.into_element();
        let to = to.into_element();
        let message_id = message_id.into_element();
        let relates_to = relates_to.map(|r| r.into_element());
        let reply_to = reply_to.map(|r| r.into_element());
        let fault_to = fault_to.map(|r| r.into_element());
        let from = from.map(|r| r.into_element());

        let elements = [
            Some(action),
            Some(to),
            Some(message_id),
            relates_to,
            reply_to,
            fault_to,
            from,
        ]
        .into_iter()
        .flatten()
        .map(|node| node.set_namespace(wsa_ns!()))
        .collect::<Vec<_>>();

        elements.into_iter()
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

        // Required fields
        let action = action.ok_or(XmlError::GenericError("Action is required".into()))?;
        let to = to.ok_or(XmlError::GenericError("To is required".into()))?;
        let message_id =
            message_id.ok_or(XmlError::GenericError("MessageID is required".into()))?;

        Ok(WsAddressingHeaders {
            action: Action::new_tag1(action, MustUnderstand { value: false }),
            to: To::new_tag(to),
            message_id: MessageID::new_tag(message_id),
            relates_to: relates_to.map(|r| RelatesTo::new_tag(r)),
            reply_to: reply_to.map(|r| ReplyTo::new_tag(r)),
            fault_to: fault_to.map(|r| FaultTo::new_tag(r)),
            from: from.map(|r| From::new_tag(r)),
        })
    }
}

impl<'a> crate::soap::SoapHeaders<'a> for WsAddressingHeaders<'a> {
    const NAMESPACE: &'static str = WSA_NAMESPACE;
}
