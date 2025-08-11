use std::collections::HashMap;

use tracing::warn;
use xml::{
    builder::Element,
    parser::{XmlDeserialize, XmlVisitor},
};

use crate::cores::{self, OptionTagName, Selector, Tag, TagName, TagValue, Text};

#[derive(Debug, Clone)]
pub struct SelectorSetValue {
    pub selectors: HashMap<String, String>,
}

impl SelectorSetValue {
    pub fn new() -> Self {
        Self {
            selectors: HashMap::new(),
        }
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

impl Default for SelectorSetValue {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> TagValue<'a> for SelectorSetValue {
    fn append_to_element(self, mut element: Element<'a>) -> Element<'a> {
        for (name, value) in self.selectors {
            let selector = Tag::from_name(Selector)
                .with_value(Text::from(value))
                .with_attribute(crate::cores::Attribute::Name(name.into()));

            let selector = selector.into_element();

            element = element.add_child(selector);
        }

        element
    }
}

pub struct SelectorSetVisitor {
    selectors: HashMap<String, String>,
}

impl<'a> XmlVisitor<'a> for SelectorSetVisitor {
    type Value = SelectorSetValue;

    fn visit_children(
        &mut self,
        children: impl Iterator<Item = xml::parser::Node<'a, 'a>>,
    ) -> Result<(), xml::XmlError> {
        for child in children {
            if !child.is_element() {
                continue;
            }

            match (child.tag_name().name(), child.tag_name().namespace()) {
                (Selector::TAG_NAME, Selector::NAMESPACE) => {
                    // Extract Name attribute and text content
                    let mut name = None;
                    for attr in child.attributes() {
                        if attr.name() == "Name" {
                            name = Some(attr.value().to_string());
                            break;
                        }
                    }

                    if let Some(name) = name {
                        let value = child.text().unwrap_or_default().to_string();
                        self.selectors.insert(name, value);
                    } else {
                        warn!("Selector element missing Name attribute");
                    }
                }
                _ => {
                    warn!(
                        "Unexpected child element in SelectorSetValue: {} (namespace: {:?})",
                        child.tag_name().name(),
                        child.tag_name().namespace()
                    );
                }
            }
        }

        Ok(())
    }

    fn visit_node(&mut self, _node: xml::parser::Node<'a, 'a>) -> Result<(), xml::XmlError> {
        // SelectorSetValue doesn't need to process individual nodes, only children
        Ok(())
    }

    fn finish(self) -> Result<Self::Value, xml::XmlError> {
        Ok(SelectorSetValue {
            selectors: self.selectors,
        })
    }
}

impl<'a> XmlDeserialize<'a> for SelectorSetValue {
    type Visitor = SelectorSetVisitor;

    fn visitor() -> Self::Visitor {
        SelectorSetVisitor {
            selectors: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct OptionSetValue {
    pub options: HashMap<String, String>,
}

impl Default for OptionSetValue {
    fn default() -> Self {
        Self::new()
    }
}

impl OptionSetValue {
    pub fn new() -> Self {
        Self {
            options: HashMap::new(),
        }
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
                .set_namespace(xml::builder::Namespace::from(
                    OptionTagName::NAMESPACE.expect("OptionTagName definately has a namespace"),
                ))
                .set_text(value)
                .add_attribute(cores::Attribute::Name(name.into()).into())
                .add_attribute(cores::Attribute::MustComply(true).into());
            element = element.add_child(option_element);
        }

        element
    }
}

pub struct OptionSetVisitor {
    options: HashMap<String, String>,
}

impl<'a> XmlVisitor<'a> for OptionSetVisitor {
    type Value = OptionSetValue;

    fn visit_children(
        &mut self,
        children: impl Iterator<Item = xml::parser::Node<'a, 'a>>,
    ) -> Result<(), xml::XmlError> {
        for child in children {
            if !child.is_element() {
                continue;
            }

            match (child.tag_name().name(), child.tag_name().namespace()) {
                (OptionTagName::TAG_NAME, OptionTagName::NAMESPACE) => {
                    // Extract Name attribute and text content
                    let mut name = None;
                    for attr in child.attributes() {
                        if attr.name() == "Name" {
                            name = Some(attr.value().to_string());
                            break;
                        }
                    }

                    if let Some(name) = name {
                        let value = child.text().unwrap_or_default().to_string();
                        self.options.insert(name, value);
                    } else {
                        warn!("Option element missing Name attribute");
                    }
                }
                _ => {
                    warn!(
                        "Unexpected child element in OptionSetValue: {} (namespace: {:?})",
                        child.tag_name().name(),
                        child.tag_name().namespace()
                    );
                }
            }
        }

        Ok(())
    }

    fn visit_node(&mut self, _node: xml::parser::Node<'a, 'a>) -> Result<(), xml::XmlError> {
        // OptionSetValue doesn't need to process individual nodes, only children
        Ok(())
    }

    fn finish(self) -> Result<Self::Value, xml::XmlError> {
        Ok(OptionSetValue {
            options: self.options,
        })
    }
}

impl<'a> XmlDeserialize<'a> for OptionSetValue {
    type Visitor = OptionSetVisitor;

    fn visitor() -> Self::Visitor {
        OptionSetVisitor {
            options: HashMap::new(),
        }
    }
}
