use tracing::warn;
use xml::parser::{XmlDeserialize, XmlVisitor};

use crate::cores::{Tag, TagValue, tag_name::*, tag_value::Text};

#[derive(Debug, Clone)]
pub struct AddressValue<'a> {
    pub url: Tag<'a, Text<'a>, Address>,
}

impl<'a> TagValue<'a> for AddressValue<'a> {
    fn append_to_element(self, element: xml::builder::Element<'a>) -> xml::builder::Element<'a> {
        let inner_element = self.url.into_element();
        element.add_child(inner_element)
    }
}

pub struct AddressVisitor<'a> {
    address: Option<AddressValue<'a>>,
}

impl<'a> XmlVisitor<'a> for AddressVisitor<'a> {
    type Value = AddressValue<'a>;

    fn visit_children(
        &mut self,
        children: impl Iterator<Item = xml::parser::Node<'a, 'a>>,
    ) -> Result<(), xml::XmlError> {
        for child in children {
            if !child.is_element() {
                continue;
            }

            match (child.tag_name().name(), child.tag_name().namespace()) {
                (Address::TAG_NAME, Address::NAMESPACE) => {
                    let tag = Tag::from_node(child)?;
                    self.address = Some(AddressValue { url: tag });
                }
                _ => {
                    warn!(
                        "Unexpected child element in AddressValue: {}",
                        child.tag_name().name()
                    );
                }
            }
        }

        Ok(())
    }

    fn visit_node(&mut self, _node: xml::parser::Node<'a, 'a>) -> Result<(), xml::XmlError> {
        todo!()
    }

    fn finish(self) -> Result<Self::Value, xml::XmlError> {
        todo!()
    }
}

impl<'a> XmlDeserialize<'a> for AddressValue<'a> {
    type Visitor = AddressVisitor<'a>;

    fn visitor() -> Self::Visitor {
        AddressVisitor { address: None }
    }
}
