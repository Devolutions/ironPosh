pub use roxmltree::*;

impl<'a> TryFrom<roxmltree::Node<'a, 'a>> for crate::builder::Element<'a> {
    type Error = crate::XmlError<'a>;

    fn try_from(value: roxmltree::Node<'a, 'a>) -> Result<Self, Self::Error> {
        if !value.is_element() {
            return Err(crate::XmlError::InvalidNodeType {
                expected: NodeType::Element,
                found: value.node_type(),
            });
        }

        let tag_name = value.tag_name();
        let namespace = tag_name
            .namespace()
            .map(|ns| crate::builder::Namespace::new(ns));

        let name = tag_name.name();

        let mut element = crate::builder::Element::new(name);

        element = element.set_namespace_optional(namespace);

        Ok(element)
    }
}

pub fn parse<'a>(xml: &'a str) -> Result<Document<'a>, roxmltree::Error> {
    roxmltree::Document::parse(xml)
}
