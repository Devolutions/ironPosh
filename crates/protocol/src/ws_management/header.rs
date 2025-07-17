use std::collections::HashSet;

use xml::builder::Element;

use crate::{
    define_tagname, push_element,
    traits::{
        DeclareNamespaces, MustUnderstand, Tag, Tag1, TagValue, namespace::RspShellNamespaceAlias,
        tag_value::Text,
    },
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
define_tagname!(SessionId, Some(WSMAN_NAMESPACE));
define_tagname!(CompressionType, Some(WSMAN_NAMESPACE));
define_tagname!(OptionSet, Some(WSMAN_NAMESPACE));

#[derive(Debug, Clone)]
pub struct SelectorSetValue<'a> {
    selectors: HashSet<Text<'a>>,
}

impl<'a> SelectorSetValue<'a> {
    pub fn new(selectors: HashSet<Text<'a>>) -> Self {
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
    options: HashSet<Text<'a>>,
}

impl<'a> OptionSetValue<'a> {
    pub fn new(options: HashSet<Text<'a>>) -> Self {
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
    #[builder(default, setter(strip_option, into))]
    pub resource_uri: Option<Tag1<'a, Text<'a>, ResourceURI, MustUnderstand>>,
    #[builder(default, setter(strip_option, into))]
    pub selector_set: Option<Tag<'a, SelectorSetValue<'a>, SelectorSet>>,
    #[builder(default, setter(strip_option, into))]
    pub option_set: Option<Tag<'a, OptionSetValue<'a>, OptionSet>>,
    #[builder(default, setter(strip_option, into))]
    pub operation_timeout: Option<Tag<'a, Text<'a>, OperationTimeout>>,
    #[builder(default, setter(strip_option, into))]
    pub max_envelope_size: Option<Tag<'a, Text<'a>, MaxEnvelopeSize>>,
    #[builder(default, setter(strip_option, into))]
    pub locale: Option<Tag<'a, Text<'a>, Locale>>,
    #[builder(default, setter(strip_option, into))]
    pub data_locale: Option<Tag<'a, Text<'a>, DataLocale>>,
    #[builder(default, setter(strip_option, into))]
    pub sequence_id: Option<Tag<'a, Text<'a>, SequenceId>>,
    #[builder(default, setter(strip_option, into))]
    pub operation_id: Option<Tag<'a, Text<'a>, OperationID>>,
    #[builder(default, setter(strip_option, into))]
    pub fragment_transfer: Option<Tag<'a, Text<'a>, FragmentTransfer>>,
    #[builder(default, setter(strip_option, into))]
    pub session_id: Option<Tag1<'a, Text<'a>, SessionId, MustUnderstand>>,
    #[builder(default, setter(strip_option, into))]
    pub compression_type: Option<
        DeclareNamespaces<
            'a,
            RspShellNamespaceAlias,
            Tag1<'a, Text<'a>, CompressionType, MustUnderstand>,
        >,
    >,
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
            session_id,
            compression_type,
        } = self;

        let mut elements = Vec::new();

        push_element!(
            elements,
            [
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
                session_id,
                compression_type
            ]
        );

        elements.into_iter()
    }
}

impl<'a> crate::soap::SoapHeaders<'a> for WsManagementHeader<'a> {
    const NAMESPACE: &'static str = WSMAN_NAMESPACE;
}
