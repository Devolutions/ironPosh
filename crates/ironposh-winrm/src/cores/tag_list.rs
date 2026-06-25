use ironposh_xml::{builder::Element, mapping::FromXml};

use crate::cores::{TagValue, anytag::AnyTag};

// This is just a temporary struct to hold a list of tags.
// to replace the actual TagValue going to be implemented for tags
#[derive(Debug, Clone, Default)]
pub struct TagList<'a> {
    items: Vec<crate::cores::anytag::AnyTag<'a>>,
}

impl<'a> TagList<'a> {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_tag(&mut self, tag: AnyTag<'a>) {
        self.items.push(tag);
    }

    pub fn with_tag(mut self, tag: AnyTag<'a>) -> Self {
        self.add_tag(tag);
        self
    }

    pub fn inner(self) -> Vec<AnyTag<'a>> {
        self.items
    }
}

impl<'a> TagValue<'a> for TagList<'a> {
    fn append_to_element(self, element: Element<'a>) -> Element<'a> {
        element.add_children(self.items.into_iter().map(AnyTag::into_element).collect())
    }
}

impl<'a> FromXml<'a> for TagList<'a> {
    fn from_xml(node: ironposh_xml::parser::Node<'a, 'a>) -> Result<Self, ironposh_xml::XmlError> {
        let mut items = Vec::new();
        for child in node.children() {
            if child.is_element() {
                items.push(AnyTag::from_xml(child)?);
            }
        }
        Ok(TagList { items })
    }
}
