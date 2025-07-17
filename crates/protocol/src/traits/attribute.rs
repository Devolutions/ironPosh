#[derive(Debug, Clone)]
pub enum Attribute {
    MustUnderstand(bool),
}

pub struct AttributeVisitor {
    attribute: Option<Attribute>,
}

impl<'a> xml::parser::XmlVisitor<'a> for AttributeVisitor {
    type Value = Attribute;

    fn visit_children(
        &mut self,
        _node: impl Iterator<Item = xml::parser::Node<'a, 'a>>,
    ) -> Result<(), xml::XmlError<'a>> {
        Err(xml::XmlError::InvalidXml(
            "Expected no children for Attribute".to_string(),
        ))
    }

    fn visit_node(&mut self, node: xml::parser::Node<'a, 'a>) -> Result<(), xml::XmlError<'a>> {
        let mut attr = None;
        for attribute in node.attributes() {
            match attribute.name() {
                "MustUnderstand" => {
                    if let Ok(value) = attribute.value().parse::<bool>() {
                        attr = Some(Attribute::MustUnderstand(value));
                    } else {
                        return Err(xml::XmlError::InvalidXml(
                            "Invalid value for MustUnderstand".to_string(),
                        ));
                    }
                }
                _ => {
                    continue;
                }
            }
        }

        if let Some(attribute) = attr {
            self.attribute = Some(attribute);
        } else {
            return Err(xml::XmlError::InvalidXml(
                "No valid attribute found".to_string(),
            ));
        }
        Ok(())
    }

    fn finish(self) -> Result<Self::Value, xml::XmlError<'a>> {
        self.attribute
            .ok_or(xml::XmlError::InvalidXml("No attribute found".to_string()))
    }
}

impl<'a> xml::parser::XmlDeserialize<'a> for Attribute {
    type Visitor = AttributeVisitor;

    fn visitor() -> Self::Visitor {
        AttributeVisitor { attribute: None }
    }

    fn from_node(node: xml::parser::Node<'a, 'a>) -> Result<Self, xml::XmlError<'a>> {
        xml::parser::NodeDeserializer::new(node).deserialize(Self::visitor())
    }
}
