use std::collections::HashSet;

use tracing::warn;
use xml::{
    builder::Element,
    parser::{XmlDeserialize, XmlVisitor},
};

use crate::cores::{Attribute, OptionTagName, Tag, TagName, TagValue, tag_value::Text};

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
    fn append_to_element(self, element: Element<'a>) -> Element<'a> {
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
    pub options: Vec<Tag<'a, Text<'a>, OptionTagName>>,
}

impl<'a> OptionSetValue<'a> {
    pub fn new() -> Self {
        Self {
            options: Vec::new(),
        }
    }

    /// Name as attribute Name
    /// Example:
    /// <wsman:Option Name="WINRS_CONSOLEMODE_STDIN">TRUE</wsman:Option>
    /// in which value is the text of 'True'
    pub fn add_option(mut self, name: &'a str, value: &'a str, must_comply: Option<bool>) -> Self {
        let mut tag: Tag<'a, Text<'a>, OptionTagName> =
            Tag::new(Text::from(value)).with_attribute(Attribute::Name(name.into()));

        if let Some(must_comply) = must_comply {
            tag = tag.with_attribute(Attribute::MustComply(must_comply));
        }

        self.options.push(tag);
        self
    }
}

impl<'a> TagValue<'a> for OptionSetValue<'a> {
    fn append_to_element(self, mut element: Element<'a>) -> Element<'a> {
        for tag in self.options {
            let child_element = tag.into_element();
            element = element.add_child(child_element);
        }

        element
    }
}

pub struct OptionSetVisitor<'a> {
    options: Vec<Tag<'a, Text<'a>, OptionTagName>>,
}

impl<'a> XmlVisitor<'a> for OptionSetVisitor<'a> {
    type Value = OptionSetValue<'a>;

    fn visit_children(
        &mut self,
        children: impl Iterator<Item = xml::parser::Node<'a, 'a>>,
    ) -> Result<(), xml::XmlError<'a>> {
        for child in children {
            if !child.is_element() {
                continue;
            }

            match (child.tag_name().name(), child.tag_name().namespace()) {
                (OptionTagName::TAG_NAME, OptionTagName::NAMESPACE) => {
                    // Parse this Option element as a Tag
                    let option_tag = Tag::<Text<'a>, OptionTagName>::from_node(child)?;
                    self.options.push(option_tag);
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

    fn visit_node(&mut self, _node: xml::parser::Node<'a, 'a>) -> Result<(), xml::XmlError<'a>> {
        // OptionSetValue doesn't need to process individual nodes, only children
        Ok(())
    }

    fn finish(self) -> Result<Self::Value, xml::XmlError<'a>> {
        Ok(OptionSetValue {
            options: self.options,
        })
    }
}

impl<'a> XmlDeserialize<'a> for OptionSetValue<'a> {
    type Visitor = OptionSetVisitor<'a>;

    fn visitor() -> Self::Visitor {
        OptionSetVisitor {
            options: Vec::new(),
        }
    }
}
