use xml::{
    builder::Element,
    parser::{XmlDeserialize, XmlVisitor},
};

use crate::traits::{TagValue, anytag::AnyTag};

#[derive(Debug, Clone)]
pub struct TagList<'a> {
    items: Vec<crate::traits::anytag::AnyTag<'a>>,
}

impl<'a> TagValue<'a> for TagList<'a> {
    fn into_element(self, name: &'static str, namespace: Option<&'static str>) -> Element<'a> {
        

        Element::new(name)
            .set_namespace_optional(namespace)
            .add_children(
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
    ) -> Result<(), xml::XmlError<'a>> {
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

    fn visit_node(&mut self, _node: xml::parser::Node<'a, 'a>) -> Result<(), xml::XmlError<'a>> {
        Err(xml::XmlError::InvalidXml(
            "TagListVisitor should not be called with a single node".to_string(),
        ))
    }

    fn finish(self) -> Result<Self::Value, xml::XmlError<'a>> {
        Ok(TagList { items: self.items })
    }
}

impl<'a> XmlDeserialize<'a> for TagList<'a> {
    type Visitor = TagListVisitor<'a>;

    fn visitor() -> Self::Visitor {
        TagListVisitor { items: Vec::new() }
    }
}
