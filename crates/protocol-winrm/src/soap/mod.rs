pub mod body;
pub mod header;
pub mod parsing;

use xml::parser::{Document, XmlDeserialize, XmlVisitor};

use crate::{
    cores::{Tag, TagValue, tag_name::*},
    soap::{body::SoapBody, header::SoapHeaders},
};

#[derive(Debug, Clone, typed_builder::TypedBuilder)]
pub struct SoapEnvelope<'a> {
    #[builder(default, setter(into, strip_option))]
    pub header: Option<Tag<'a, SoapHeaders<'a>, Header>>,
    #[builder(setter(into))]
    pub body: Tag<'a, SoapBody<'a>, Body>,
}

impl<'a> TagValue<'a> for SoapEnvelope<'a> {
    fn append_to_element(self, element: xml::builder::Element<'a>) -> xml::builder::Element<'a> {
        let envelope = element;

        if let Some(header) = self.header {
            envelope.add_child(header.into_element())
        } else {
            envelope
        }
        .add_child(self.body.into_element())
    }
}

pub struct SoapEnvelopeVisitor<'a> {
    pub header: Option<Tag<'a, SoapHeaders<'a>, Header>>,
    pub body: Option<Tag<'a, SoapBody<'a>, Body>>,
}

impl<'a> XmlVisitor<'a> for SoapEnvelopeVisitor<'a> {
    type Value = SoapEnvelope<'a>;

    fn visit_children(
        &mut self,
        node: impl Iterator<Item = xml::parser::Node<'a, 'a>>,
    ) -> Result<(), xml::XmlError> {
        Err(xml::XmlError::InvalidXml(format!(
            "Expected a single envelope, found {} children",
            node.count()
        )))
    }

    fn visit_node(&mut self, node: xml::parser::Node<'a, 'a>) -> Result<(), xml::XmlError> {
        // Remove the is_root() check as it prevents parsing document root elements
        // The node should be an Envelope element regardless of its root status

        let header: Option<Tag<'_, SoapHeaders<'a>, Header>> = node
            .children()
            .find(|child| child.tag_name().name() == Header::TAG_NAME)
            .map(|child| {
                Tag::from_node(child).map_err(|e| xml::XmlError::InvalidXml(e.to_string()))
            })
            .transpose()?;

        // Header can be none
        self.header = header;

        let body: Option<Tag<'_, SoapBody<'a>, Body>> = node
            .children()
            .find(|child| child.tag_name().name() == Body::TAG_NAME)
            .map(|child| {
                Tag::from_node(child).map_err(|e| xml::XmlError::InvalidXml(e.to_string()))
            })
            .transpose()?;

        if body.is_none() {
            return Err(xml::XmlError::InvalidXml(
                "SoapEnvelope must contain a Body element".to_string(),
            ));
        }

        self.body = body;

        Ok(())
    }

    fn finish(self) -> Result<Self::Value, xml::XmlError> {
        Ok(SoapEnvelope {
            header: self.header,
            body: self
                .body
                .ok_or_else(|| xml::XmlError::InvalidXml("Missing Soap Body".to_string()))?,
        })
    }
}

impl<'a> XmlDeserialize<'a> for SoapEnvelope<'a> {
    type Visitor = SoapEnvelopeVisitor<'a>;

    fn visitor() -> Self::Visitor {
        SoapEnvelopeVisitor {
            header: None,
            body: None,
        }
    }
}
