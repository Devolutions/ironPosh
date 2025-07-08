use xml_builder::Element;

use crate::soap::Header;

pub const WSA_NAMESPACE: &str = "http://www.w3.org/2005/08/addressing";
pub const WSA_NAMESPACE_ALIAS: &str = "a";

pub fn headers_builder<'a>() -> WsAddressingHeadersBuilder<'a> {
    WsAddressingHeaders::builder()
}

#[derive(typed_builder::TypedBuilder, Debug, Clone)]
pub struct WsAddressingHeaders<'a> {
    #[builder(setter(into))]
    pub action: Header<'a, &'a str>,
    #[builder(setter(into))]
    pub to: Header<'a, &'a str>,
    #[builder(setter(into))]
    pub message_id: Header<'a, &'a str>,

    #[builder(default, setter(into))]
    pub relates_to: Option<Header<'a, &'a str>>,
    #[builder(default, setter(into))]
    pub reply_to: Option<Header<'a, &'a str>>,
    #[builder(default, setter(into))]
    pub fault_to: Option<Header<'a, &'a str>>,
    #[builder(default, setter(into))]
    pub from: Option<Header<'a, &'a str>>,
}

impl<'a> IntoIterator for WsAddressingHeaders<'a> {
    type Item = xml_builder::Element<'a>;

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
        .map(Into::into)
        .map(|element: Element<'_>| {
            element.set_namespace(xml_builder::Namespace::new(
                WSA_NAMESPACE_ALIAS,
                WSA_NAMESPACE,
            ))
        })
        .collect::<Vec<_>>();

        elements.into_iter()
    }
}

impl<'a> crate::soap::SoapHeaders<'a> for WsAddressingHeaders<'a> {}
