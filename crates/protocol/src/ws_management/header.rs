use std::collections::HashSet;

use xml::builder::Element;

use crate::traits::{TagValue, tag_value::Text};

// pub fn headers_builder<'a>() -> WsManagementHeaderBuilder<'a> {
//     WsManagementHeader::builder()
// }

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
        let mut element = Element::new(name).set_namespace_optional(namespace);

        for selector in self.selectors {
            element = element.add_child(
                Element::new("Selector")
                    // .set_namespace(wsman_ns!())
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
                    // .set_namespace(wsman_ns!())
                    .set_text(option),
            );
        }

        element
    }
}

// #[derive(typed_builder::TypedBuilder, Debug, Clone)]
// pub struct WsManagementHeader<'a> {
//     #[builder(default, setter(strip_option, into))]
//     pub resource_uri: Option<Tag<'a, Text<'a>, ResourceURI>>,
//     #[builder(default, setter(strip_option, into))]
//     pub selector_set: Option<Tag<'a, SelectorSetValue<'a>, SelectorSet>>,
//     // TODO: Implement
//     #[builder(default, setter(strip_option, into))]
//     pub option_set: Option<Tag<'a, TagList<'a>, OptionSet>>,
//     #[builder(default, setter(strip_option, into))]
//     pub operation_timeout: Option<Tag<'a, Text<'a>, OperationTimeout>>,
//     #[builder(default, setter(strip_option, into))]
//     pub max_envelope_size: Option<Tag<'a, Text<'a>, MaxEnvelopeSize>>,
//     #[builder(default, setter(strip_option, into))]
//     pub locale: Option<Tag<'a, Text<'a>, Locale>>,
//     #[builder(default, setter(strip_option, into))]
//     pub data_locale: Option<Tag<'a, Text<'a>, DataLocale>>,
//     #[builder(default, setter(strip_option, into))]
//     pub sequence_id: Option<Tag<'a, Text<'a>, SequenceId>>,
//     #[builder(default, setter(strip_option, into))]
//     pub operation_id: Option<Tag<'a, Text<'a>, OperationID>>,
//     #[builder(default, setter(strip_option, into))]
//     pub fragment_transfer: Option<Tag<'a, Text<'a>, FragmentTransfer>>,
//     #[builder(default, setter(strip_option, into))]
//     pub session_id: Option<Tag<'a, Text<'a>, SessionId>>,
//     #[builder(default, setter(strip_option, into))]
//     pub compression_type: Option<Tag<'a, TagList<'a>, CompressionType>>,
// }

// impl<'a> IntoIterator for WsManagementHeader<'a> {
//     type Item = xml::builder::Element<'a>;

//     type IntoIter = std::vec::IntoIter<Self::Item>;

//     fn into_iter(self) -> Self::IntoIter {
//         let WsManagementHeader {
//             resource_uri,
//             selector_set,
//             option_set,
//             operation_timeout,
//             max_envelope_size,
//             locale,
//             data_locale,
//             sequence_id,
//             operation_id,
//             fragment_transfer,
//             session_id,
//             compression_type,
//         } = self;

//         let mut elements = Vec::new();

//         push_elements!(
//             elements,
//             [
//                 resource_uri,
//                 selector_set,
//                 option_set,
//                 operation_timeout,
//                 max_envelope_size,
//                 locale,
//                 data_locale,
//                 sequence_id,
//                 operation_id,
//                 fragment_transfer,
//                 session_id,
//                 compression_type
//             ]
//         );

//         elements.into_iter()
//     }
// }
