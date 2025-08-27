use ironposh_macros::{SimpleTagValue, SimpleXmlDeserialize};

use crate::cores::{Tag, tag_name::*, tag_value::Text};

#[derive(Debug, Clone, SimpleTagValue, SimpleXmlDeserialize)]
pub struct AddressValue<'a> {
    pub url: Tag<'a, Text<'a>, Address>,
}

// impl<'a> TagValue<'a> for AddressValue<'a> {
//     fn append_to_element(self, element: ironposh_xml::builder::Element<'a>) -> ironposh_xml::builder::Element<'a> {
//         let inner_element = self.url.into_element();
//         element.add_child(inner_element)
//     }
// }

// pub struct AddressVisitor<'a> {
//     address: Option<AddressValue<'a>>,
// }

// impl<'a> XmlVisitor<'a> for AddressVisitor<'a> {
//     type Value = AddressValue<'a>;

//     fn visit_children(
//         &mut self,
//         children: impl Iterator<Item = ironposh_xml::parser::Node<'a, 'a>>,
//     ) -> Result<(), ironposh_xml::XmlError> {
//         for child in children {
//             if !child.is_element() {
//                 continue;
//             }

//             match (child.tag_name().name(), child.tag_name().namespace()) {
//                 (Address::TAG_NAME, Address::NAMESPACE) => {
//                     let tag = Tag::from_node(child)?;
//                     self.address = Some(AddressValue { url: tag });
//                 }
//                 _ => {
//                     warn!(
//                         "Unexpected child element in AddressValue: {}",
//                         child.tag_name().name()
//                     );
//                 }
//             }
//         }

//         Ok(())
//     }

//     fn finish(self) -> Result<Self::Value, ironposh_xml::XmlError> {
//         Ok(AddressValue {
//             url: self.address.ok_or_else(|| {
//                 ironposh_xml::XmlError::NotSupposeToBeCalled {
//                     extra_info: "AddressValue must contain an Address element".to_string(),
//                 }
//             }?),
//         })
//     }
// }

// impl<'a> XmlDeserialize<'a> for AddressValue<'a> {
//     type Visitor = AddressVisitor<'a>;

//     fn visitor() -> Self::Visitor {
//         AddressVisitor { address: None }
//     }
// }
