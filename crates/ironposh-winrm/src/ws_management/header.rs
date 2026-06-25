use std::collections::HashMap;

use ironposh_xml::{
    builder::Element,
    mapping::{FromXml, NodeExt},
};

use crate::cores::tag_value::leaf_text;
use crate::cores::{self, OptionTagNameTag, Selector, SelectorTag, TagName, TagValue, Text};
use crate::tag;

tag!(SelectorSet = SelectorSetValue => DmtfWsmanSchema);
tag!(OptionSet = OptionSetValue => DmtfWsmanSchema);

#[derive(Debug, Clone, Default)]
pub struct SelectorSetValue {
    pub selectors: HashMap<String, String>,
}

impl SelectorSetValue {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a selector as a key-value pair
    /// Example:
    /// selector_set.add_selector("ShellId", "12345-67890")
    /// Generates: <w:Selector Name="ShellId">12345-67890</w:Selector>
    pub fn add_selector(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.selectors.insert(name.into(), value.into());
        self
    }

    /// Add a selector using a mutable reference for chaining
    pub fn insert_selector(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.selectors.insert(name.into(), value.into());
    }

    pub fn get(&self, name: &str) -> Option<&String> {
        self.selectors.get(name)
    }
}

impl<'a> TagValue<'a> for SelectorSetValue {
    fn append_to_element(self, mut element: Element<'a>) -> Element<'a> {
        for (name, value) in self.selectors {
            let selector = Selector::new(Text::from(value))
                .with_attribute(crate::cores::Attribute::Name(name.into()));

            let selector = selector.into_element();

            element = element.add_child(selector);
        }

        element
    }
}

impl<'a> FromXml<'a> for SelectorSetValue {
    fn from_xml(node: ironposh_xml::parser::Node<'a, 'a>) -> Result<Self, ironposh_xml::XmlError> {
        let mut selectors = HashMap::new();
        for child in node.children() {
            if child.is_element_named(SelectorTag::NAMESPACE, SelectorTag::TAG_NAME)
                && let Some(name) = child
                    .attributes()
                    .find(|attr| attr.name() == "Name")
                    .map(|attr| attr.value().to_string())
            {
                if selectors.contains_key(&name) {
                    return Err(ironposh_xml::XmlError::InvalidXml(format!(
                        "duplicate selector {name:?}"
                    )));
                }
                selectors.insert(name, leaf_text(child)?.to_string());
            }
        }
        Ok(Self { selectors })
    }
}

#[derive(Debug, Clone, Default)]
pub struct OptionSetValue {
    pub options: HashMap<String, String>,
}

impl OptionSetValue {
    pub fn new() -> Self {
        Self::default()
    }

    /// Add an option as a key-value pair
    /// Example:
    /// option_set.add_option("WINRS_CONSOLEMODE_STDIN", "TRUE")
    /// Generates: <w:Option Name="WINRS_CONSOLEMODE_STDIN">TRUE</w:Option>
    pub fn add_option(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.options.insert(name.into(), value.into());
        self
    }

    /// Add an option using a mutable reference for chaining
    pub fn insert_option(&mut self, name: impl Into<String>, value: impl Into<String>) {
        self.options.insert(name.into(), value.into());
    }
}

impl<'a> TagValue<'a> for OptionSetValue {
    fn append_to_element(self, mut element: Element<'a>) -> Element<'a> {
        for (name, value) in self.options {
            let option_element = Element::new("Option")
                .set_namespace(ironposh_xml::builder::Namespace::from(
                    OptionTagNameTag::NAMESPACE.expect("OptionTagName definately has a namespace"),
                ))
                .set_text(value)
                .add_attribute(cores::Attribute::Name(name.into()).into())
                .add_attribute(cores::Attribute::MustComply(true).into());
            element = element.add_child(option_element);
        }

        element
    }
}

impl<'a> FromXml<'a> for OptionSetValue {
    fn from_xml(node: ironposh_xml::parser::Node<'a, 'a>) -> Result<Self, ironposh_xml::XmlError> {
        let mut options = HashMap::new();
        for child in node.children() {
            if child.is_element_named(OptionTagNameTag::NAMESPACE, OptionTagNameTag::TAG_NAME)
                && let Some(name) = child
                    .attributes()
                    .find(|attr| attr.name() == "Name")
                    .map(|attr| attr.value().to_string())
            {
                if options.contains_key(&name) {
                    return Err(ironposh_xml::XmlError::InvalidXml(format!(
                        "duplicate option {name:?}"
                    )));
                }
                options.insert(name, leaf_text(child)?.to_string());
            }
        }
        Ok(Self { options })
    }
}
