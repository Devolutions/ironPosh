use std::collections::HashSet;

use xml::{XmlError, builder::Element, parser::Node};

use crate::{
    must_be_text,
    soap::{Header, Value},
    ws_management::WSMAN_NAMESPACE,
    wsman_ns,
};

pub fn headers_builder<'a>() -> WsManagementHeaderBuilder<'a> {
    WsManagementHeader::builder()
}

#[derive(Debug, Clone)]
pub struct SelectorSet<'a> {
    selectors: HashSet<&'a str>,
}

impl<'a> SelectorSet<'a> {
    pub fn new(selectors: HashSet<&'a str>) -> Self {
        Self { selectors }
    }
}

impl<'a> Value<'a> for SelectorSet<'a> {
    fn into_element(self, name: &'static str) -> Element<'a> {
        let mut element = Element::new(name).set_namespace(wsman_ns!());

        for selector in self.selectors {
            element = element.add_child(
                Element::new("Selector")
                    .set_namespace(wsman_ns!())
                    .set_text(selector),
            );
        }

        element
    }
}

#[derive(Debug, Clone)]
pub struct OptionSet<'a> {
    options: HashSet<&'a str>,
}

impl<'a> OptionSet<'a> {
    pub fn new(options: HashSet<&'a str>) -> Self {
        Self { options }
    }
}

impl<'a> Value<'a> for OptionSet<'a> {
    fn into_element(self, name: &'static str) -> Element<'a> {
        let mut element = Element::new(name).set_namespace(wsman_ns!());

        for option in self.options {
            element = element.add_child(
                Element::new("Option")
                    .set_namespace(wsman_ns!())
                    .set_text(option),
            );
        }

        element
    }
}

#[derive(typed_builder::TypedBuilder, Debug, Clone)]
pub struct WsManagementHeader<'a> {
    #[builder(setter(into))]
    pub resource_uri: Option<Header<'a, &'a str>>, // This should be a set to allow multiple URIs
    #[builder(default, setter(into))]
    pub selector_set: Option<Header<'a, SelectorSet<'a>>>,
    #[builder(default, setter(into))]
    pub option_set: Option<Header<'a, OptionSet<'a>>>, // TODO: Implement as a complex type if needed
    #[builder(default, setter(into))]
    pub operation_timeout: Option<Header<'a, &'a str>>,
    #[builder(default, setter(into))]
    pub max_envelope_size: Option<Header<'a, &'a str>>,
    #[builder(default, setter(into))]
    pub locale: Option<Header<'a, &'a str>>,
    #[builder(default, setter(into))]
    pub data_locale: Option<Header<'a, &'a str>>,
    #[builder(default, setter(into))]
    pub sequence_id: Option<Header<'a, &'a str>>,
    #[builder(default, setter(into))]
    pub operation_id: Option<Header<'a, &'a str>>,
    #[builder(default, setter(into))]
    pub fragment_transfer: Option<Header<'a, &'a str>>,
}

impl<'a> IntoIterator for WsManagementHeader<'a> {
    type Item = xml::builder::Element<'a>;

    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        let WsManagementHeader {
            resource_uri,
            selector_set,
            option_set,
            operation_timeout,
            max_envelope_size,
            locale,
            data_locale,
            sequence_id,
            operation_id,
            fragment_transfer,
        } = self;

        let resource_uri = resource_uri.map(|r| r.into_element("ResourceURI"));
        let selector_set = selector_set.map(|s| s.into_element("SelectorSet"));
        let option_set = option_set.map(|o| o.into_element("OptionSet"));
        let operation_timeout = operation_timeout.map(|o| o.into_element("OperationTimeout"));
        let max_envelope_size = max_envelope_size.map(|m| m.into_element("MaxEnvelopeSize"));
        let locale = locale.map(|l| l.into_element("Locale"));
        let data_locale = data_locale.map(|d| d.into_element("DataLocale"));
        let sequence_id = sequence_id.map(|s| s.into_element("SequenceId"));
        let operation_id = operation_id.map(|o| o.into_element("OperationID"));
        let fragment_transfer = fragment_transfer.map(|f| f.into_element("FragmentTransfer"));

        let elements = [
            resource_uri,
            selector_set,
            option_set,
            operation_timeout,
            max_envelope_size,
            locale,
            data_locale,
            sequence_id,
            operation_id,
            fragment_transfer,
        ]
        .into_iter()
        .flatten()
        .map(|e| e.set_namespace(wsman_ns!()))
        .collect::<Vec<_>>();
        elements.into_iter()
    }
}

impl<'a> crate::soap::SoapHeaders<'a> for WsManagementHeader<'a> {
    const NAMESPACE: &'static str = WSMAN_NAMESPACE;
}

impl<'a> TryFrom<Vec<Node<'a, 'a>>> for WsManagementHeader<'a> {
    type Error = xml::XmlError<'a>;

    fn try_from(value: Vec<Node<'a, 'a>>) -> Result<Self, Self::Error> {
        let mut resource_uri = None;
        let mut selector_set = None;
        let mut option_set = None;
        let mut operation_timeout = None;
        let mut max_envelope_size = None;
        let mut locale = None;
        let mut data_locale = None;
        let mut sequence_id = None;
        let mut operation_id = None;
        let mut fragment_transfer = None;

        for node in value {
            match node.tag_name().name() {
                "ResourceURI" => {
                    let value = {
                        let child = node
                            .first_child()
                            .ok_or(XmlError::GenericError("expecting node".into()))?;
                        must_be_text!(child);
                        let text = child.text().expect("must be text");
                        text
                    };
                    resource_uri = Some(Header::from(value.trim()));
                }
                "SelectorSet" => {
                    let mut selectors = HashSet::new();
                    for child in node.children() {
                        if child.tag_name().name() == "Selector" {
                            if let Some(text_child) = child.first_child() {
                                must_be_text!(text_child);
                                if let Some(text) = text_child.text() {
                                    selectors.insert(text.trim());
                                }
                            }
                        }
                    }
                    selector_set = Some(Header::from(SelectorSet::new(selectors)));
                }
                "OptionSet" => {
                    let mut options = HashSet::new();
                    for child in node.children() {
                        if child.tag_name().name() == "Option" {
                            if let Some(text_child) = child.first_child() {
                                must_be_text!(text_child);
                                if let Some(text) = text_child.text() {
                                    options.insert(text.trim());
                                }
                            }
                        }
                    }
                    option_set = Some(Header::from(OptionSet::new(options)));
                }
                "OperationTimeout" => {
                    let value = {
                        let child = node
                            .first_child()
                            .ok_or(XmlError::GenericError("expecting node".into()))?;
                        must_be_text!(child);
                        let text = child.text().expect("must be text");
                        text
                    };
                    operation_timeout = Some(Header::from(value.trim()));
                }
                "MaxEnvelopeSize" => {
                    let value = {
                        let child = node
                            .first_child()
                            .ok_or(XmlError::GenericError("expecting node".into()))?;
                        must_be_text!(child);
                        let text = child.text().expect("must be text");
                        text
                    };
                    max_envelope_size = Some(Header::from(value.trim()));
                }
                "Locale" => {
                    let value = {
                        let child = node
                            .first_child()
                            .ok_or(XmlError::GenericError("expecting node".into()))?;
                        must_be_text!(child);
                        let text = child.text().expect("must be text");
                        text
                    };
                    locale = Some(Header::from(value.trim()));
                }
                "DataLocale" => {
                    let value = {
                        let child = node
                            .first_child()
                            .ok_or(XmlError::GenericError("expecting node".into()))?;
                        must_be_text!(child);
                        let text = child.text().expect("must be text");
                        text
                    };
                    data_locale = Some(Header::from(value.trim()));
                }
                "SequenceId" => {
                    let value = {
                        let child = node
                            .first_child()
                            .ok_or(XmlError::GenericError("expecting node".into()))?;
                        must_be_text!(child);
                        let text = child.text().expect("must be text");
                        text
                    };
                    sequence_id = Some(Header::from(value.trim()));
                }
                "OperationID" => {
                    let value = {
                        let child = node
                            .first_child()
                            .ok_or(XmlError::GenericError("expecting node".into()))?;
                        must_be_text!(child);
                        let text = child.text().expect("must be text");
                        text
                    };
                    operation_id = Some(Header::from(value.trim()));
                }
                "FragmentTransfer" => {
                    let value = {
                        let child = node
                            .first_child()
                            .ok_or(XmlError::GenericError("expecting node".into()))?;
                        must_be_text!(child);
                        let text = child.text().expect("must be text");
                        text
                    };
                    fragment_transfer = Some(Header::from(value.trim()));
                }
                tag_name => {
                    return Err(xml::XmlError::UnexpectedTag(tag_name.into()));
                }
            }
        }

        Ok(WsManagementHeader {
            resource_uri,
            selector_set,
            option_set,
            operation_timeout,
            max_envelope_size,
            locale,
            data_locale,
            sequence_id,
            operation_id,
            fragment_transfer,
        })
    }
}
