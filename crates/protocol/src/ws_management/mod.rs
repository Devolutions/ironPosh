use std::{collections::HashSet, vec};

use xml_builder::Element;

use crate::{Node, opt_header, soap::Header};

pub const WSMAN_NAMESPACE: &str = "http://schemas.dmtf.org/wbem/wsman/1/wsman.xsd";
pub const WSMAN_NAMESPACE_ALIAS: &str = "w";

macro_rules! namespace {
    () => {
        xml_builder::Namespace::new(WSMAN_NAMESPACE_ALIAS, WSMAN_NAMESPACE)
    };
}

pub fn headers_builder<'a>() -> WsManagementHeaderBuilder<'a> {
    WsManagementHeader::builder()
}

#[derive(Debug, Clone)]
pub struct ResourceUri<'a> {
    uri: &'a str,
}

impl<'a> ResourceUri<'a> {
    pub fn new(uri: &'a str) -> Self {
        Self { uri }
    }
}

impl<'a> Node<'a> for ResourceUri<'a> {
    fn into_element(self) -> Element<'a> {
        let element = Element::new("ResourceURI")
            .set_namespace(namespace!())
            .set_text(self.uri);

        element
    }
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

impl<'a> Node<'a> for SelectorSet<'a> {
    fn into_element(self) -> Element<'a> {
        let mut element = Element::new("SelectorSet").set_namespace(namespace!());

        for selector in self.selectors {
            element = element.add_child(
                Element::new("Selector")
                    .set_namespace(namespace!())
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

impl<'a> Node<'a> for OptionSet<'a> {
    fn into_element(self) -> Element<'a> {
        let mut element = Element::new("OptionSet").set_namespace(namespace!());

        for option in self.options {
            element = element.add_child(
                Element::new("Option")
                    .set_namespace(namespace!())
                    .set_text(option),
            );
        }

        element
    }
}

#[derive(typed_builder::TypedBuilder, Debug, Clone)]
pub struct WsManagementHeader<'a> {
    pub resource_uri: Header<'a, ResourceUri<'a>>, // This should be a set to allow multiple URIs
    #[builder(default)]
    pub selector_set: Option<Header<'a, SelectorSet<'a>>>,
    #[builder(default)]
    pub option_set: Option<Header<'a, OptionSet<'a>>>, // TODO: Implement as a complex type if needed
    #[builder(default)]
    pub operation_timeout: Option<Header<'a, &'a str>>,
    #[builder(default)]
    pub max_envelope_size: Option<Header<'a, &'a str>>,
    #[builder(default)]
    pub locale: Option<Header<'a, &'a str>>,
    #[builder(default)]
    pub data_locale: Option<Header<'a, &'a str>>,
    #[builder(default)]
    pub sequence_id: Option<Header<'a, &'a str>>,
    #[builder(default)]
    pub operation_id: Option<Header<'a, &'a str>>,
    #[builder(default)]
    pub fragment_transfer: Option<Header<'a, &'a str>>,
}

impl<'a> IntoIterator for WsManagementHeader<'a> {
    type Item = xml_builder::Element<'a>;

    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        let mut elements = vec![self.resource_uri.into_element()];

        opt_header!(
            elements,
            self.selector_set,
            self.option_set,
            self.operation_timeout,
            self.max_envelope_size,
            self.locale,
            self.data_locale,
            self.sequence_id,
            self.operation_id,
            self.fragment_transfer,
        );

        elements
            .into_iter()
            .map(|e| e.set_namespace(namespace!()))
            .collect::<Vec<_>>()
            .into_iter()
    }
}

impl<'a> crate::soap::SoapHeaders<'a> for WsManagementHeader<'a> {}
