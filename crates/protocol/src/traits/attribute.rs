use xml::parser::{XmlDeserialize, XmlVisitor};

pub trait Attribute<'a> {
    fn value(&self) -> Option<&'a str>;

    const NAME: &'static str;
    const NAMESPACE: Option<&'static str>;
}

#[derive(Debug, Clone)]
pub struct MustUnderstand {
    pub value: bool,
}

impl MustUnderstand {
    pub fn yes() -> Self {
        MustUnderstand { value: true }
    }

    pub fn no() -> Self {
        MustUnderstand { value: false }
    }
}

impl<'a> Attribute<'a> for MustUnderstand {
    const NAME: &'static str = "mustUnderstand";
    const NAMESPACE: Option<&'static str> = Some(crate::soap::SOAP_NAMESPACE);

    fn value(&self) -> Option<&'a str> {
        if self.value { Some("true") } else { None }
    }
}

pub struct MustUnderstandVisitor {
    value: Option<MustUnderstand>,
}

impl<'a> xml::parser::XmlVisitor<'a> for MustUnderstandVisitor {
    type Value = MustUnderstand;

    fn visit_node(&mut self, node: xml::parser::Node<'a, 'a>) -> Result<(), xml::XmlError<'a>> {
        let attributes = node.attributes();

        for attr in attributes {
            if attr.name() == MustUnderstand::NAME && attr.namespace() == MustUnderstand::NAMESPACE
            {
                match attr.value() {
                    "true" => self.value = Some(MustUnderstand::yes()),
                    "false" => self.value = Some(MustUnderstand::no()),
                    _ => {
                        return Err(xml::XmlError::InvalidXml(
                            "Invalid value for mustUnderstand attribute".to_string(),
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    fn visit_children(
        &mut self,
        _children: xml::parser::Children<'a, 'a>,
    ) -> Result<(), xml::XmlError<'a>> {
        // Attributes don't process children
        Ok(())
    }

    fn finish(self) -> Result<Self::Value, xml::XmlError<'a>> {
        self.value.ok_or(xml::XmlError::InvalidXml(
            "No mustUnderstand attribute found".to_string(),
        ))
    }
}

impl<'a> XmlDeserialize<'a> for MustUnderstand {
    type Visitor = MustUnderstandVisitor;

    fn visitor() -> Self::Visitor {
        MustUnderstandVisitor { value: None }
    }

    fn from_node(node: xml::parser::Node<'a, 'a>) -> Result<Self, xml::XmlError<'a>> {
        let mut visitor = Self::visitor();
        visitor.visit_node(node)?;
        visitor.finish()
    }
}
