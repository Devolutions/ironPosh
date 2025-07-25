use xml::{
    builder::Element,
    parser::{XmlDeserialize, XmlVisitor},
};

use crate::cores::{TagValue, anytag::AnyTag};

#[derive(Debug, Clone)]
pub struct TagList<'a> {
    items: Vec<crate::cores::anytag::AnyTag<'a>>,
}

impl<'a> Default for TagList<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> TagList<'a> {
    pub fn new() -> Self {
        Self { items: Vec::new() }
    }

    pub fn add_tag(&mut self, tag: AnyTag<'a>) {
        self.items.push(tag);
    }

    pub fn with_tag(mut self, tag: AnyTag<'a>) -> Self {
        self.add_tag(tag);
        self
    }

    pub fn into_iter(self) -> impl Iterator<Item = AnyTag<'a>> {
        self.items.into_iter()
    }
}

impl<'a> TagValue<'a> for TagList<'a> {
    fn append_to_element(self, element: Element<'a>) -> Element<'a> {
        element.add_children(
            self.items
                .into_iter()
                .map(|tag| tag.into_element())
                .collect(),
        )
    }
}

pub struct TagListVisitor<'a> {
    items: Vec<AnyTag<'a>>,
}

impl<'a> XmlVisitor<'a> for TagListVisitor<'a> {
    type Value = TagList<'a>;

    fn visit_children(
        &mut self,
        children: impl Iterator<Item = xml::parser::Node<'a, 'a>>,
    ) -> Result<(), xml::XmlError> {
        for child in children {
            if child.is_element() {
                let tag = AnyTag::from_node(child)?;
                self.items.push(tag);
            } else {
                return Err(xml::XmlError::InvalidXml(format!(
                    "Expected element child, found: {:?}",
                    child.node_type()
                )));
            }
        }
        Ok(())
    }

    fn visit_node(&mut self, _node: xml::parser::Node<'a, 'a>) -> Result<(), xml::XmlError> {
        Err(xml::XmlError::InvalidXml(
            "TagListVisitor should not be called with a single node".to_string(),
        ))
    }

    fn finish(self) -> Result<Self::Value, xml::XmlError> {
        Ok(TagList { items: self.items })
    }
}

impl<'a> XmlDeserialize<'a> for TagList<'a> {
    type Visitor = TagListVisitor<'a>;

    fn visitor() -> Self::Visitor {
        TagListVisitor { items: Vec::new() }
    }
}
