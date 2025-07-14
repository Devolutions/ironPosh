use std::collections::HashSet;

use xml::{XmlError, builder::Element, parser::Node};

use crate::{
    define_tagname, must_be_text,
    traits::{MustUnderstand, Tag, Tag1, TagValue},
    ws_management::WSMAN_NAMESPACE,
    wsman_ns,
};

pub fn headers_builder<'a>() -> WsManagementHeaderBuilder<'a> {
    WsManagementHeader::builder()
}

// Define tag names for WS-Management headers
define_tagname!(ResourceURI, Some(WSMAN_NAMESPACE));
define_tagname!(OperationTimeout, Some(WSMAN_NAMESPACE));
define_tagname!(MaxEnvelopeSize, Some(WSMAN_NAMESPACE));
define_tagname!(Locale, Some(WSMAN_NAMESPACE));
define_tagname!(DataLocale, Some(WSMAN_NAMESPACE));
define_tagname!(SequenceId, Some(WSMAN_NAMESPACE));
define_tagname!(OperationID, Some(WSMAN_NAMESPACE));
define_tagname!(FragmentTransfer, Some(WSMAN_NAMESPACE));
define_tagname!(SelectorSet, Some(WSMAN_NAMESPACE));

define_tagname!(OptionSet, Some(WSMAN_NAMESPACE));

#[derive(Debug, Clone)]
pub struct SelectorSetValue<'a> {
    selectors: HashSet<&'a str>,
}

impl<'a> SelectorSetValue<'a> {
    pub fn new(selectors: HashSet<&'a str>) -> Self {
        Self { selectors }
    }
}

impl<'a> TagValue<'a> for SelectorSetValue<'a> {
    fn into_element(self, name: &'static str, namespace: Option<&'static str>) -> Element<'a> {
        let mut element =
            Element::new(name).set_namespace_optional(namespace.or(Some(WSMAN_NAMESPACE)));

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
pub struct OptionSetValue<'a> {
    options: HashSet<&'a str>,
}

impl<'a> OptionSetValue<'a> {
    pub fn new(options: HashSet<&'a str>) -> Self {
        Self { options }
    }
}

impl<'a> TagValue<'a> for OptionSetValue<'a> {
    fn into_element(self, name: &'static str, ns: Option<&'static str>) -> Element<'a> {
        let mut element = Element::new(name);

        if let Some(ns) = ns {
            element = element.set_namespace(ns);
        }

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
    #[builder(default, setter(into))]
    pub resource_uri: Option<Tag1<'a, &'a str, ResourceURI, MustUnderstand>>,
    #[builder(default, setter(into))]
    pub selector_set: Option<Tag<'a, SelectorSetValue<'a>, SelectorSet>>,
    #[builder(default, setter(into))]
    pub option_set: Option<Tag<'a, OptionSetValue<'a>, OptionSet>>,
    #[builder(default, setter(into))]
    pub operation_timeout: Option<Tag<'a, &'a str, OperationTimeout>>,
    #[builder(default, setter(into))]
    pub max_envelope_size: Option<Tag<'a, &'a str, MaxEnvelopeSize>>,
    #[builder(default, setter(into))]
    pub locale: Option<Tag<'a, &'a str, Locale>>,
    #[builder(default, setter(into))]
    pub data_locale: Option<Tag<'a, &'a str, DataLocale>>,
    #[builder(default, setter(into))]
    pub sequence_id: Option<Tag<'a, &'a str, SequenceId>>,
    #[builder(default, setter(into))]
    pub operation_id: Option<Tag<'a, &'a str, OperationID>>,
    #[builder(default, setter(into))]
    pub fragment_transfer: Option<Tag<'a, &'a str, FragmentTransfer>>,
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

        let resource_uri = resource_uri.map(|r| r.into_element());
        println!("Resource URI: {:?}", resource_uri);
        let selector_set = selector_set.map(|s| s.into_element());
        let option_set = option_set.map(|o| o.into_element());
        let operation_timeout = operation_timeout.map(|o| o.into_element());
        let max_envelope_size = max_envelope_size.map(|m| m.into_element());
        let locale = locale.map(|l| l.into_element());
        let data_locale = data_locale.map(|d| d.into_element());
        let sequence_id = sequence_id.map(|s| s.into_element());
        let operation_id = operation_id.map(|o| o.into_element());
        let fragment_transfer = fragment_transfer.map(|f| f.into_element());

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
                    resource_uri = Some(ResourceURI::new_tag1(value.trim(), MustUnderstand::no()));
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
                    selector_set = Some(SelectorSet::new_tag(SelectorSetValue::new(selectors)));
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
                    option_set = Some(OptionSet::new_tag(OptionSetValue::new(options)));
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
                    operation_timeout = Some(OperationTimeout::new_tag(value.trim()));
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
                    max_envelope_size = Some(MaxEnvelopeSize::new_tag(value.trim()));
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
                    locale = Some(Locale::new_tag(value.trim()));
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
                    data_locale = Some(DataLocale::new_tag(value.trim()));
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
                    sequence_id = Some(SequenceId::new_tag(value.trim()));
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
                    operation_id = Some(OperationID::new_tag(value.trim()));
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
                    fragment_transfer = Some(FragmentTransfer::new_tag(value.trim()));
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
