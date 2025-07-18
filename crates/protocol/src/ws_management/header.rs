use std::collections::HashSet;

use xml::builder::Element;

use crate::cores::{TagValue, tag_value::Text};

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
    fn into_element(self, element: Element<'a>) -> Element<'a> {
        let mut element = element;

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
    fn into_element(self, element: Element<'a>) -> Element<'a> {
        let mut element = element;

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
