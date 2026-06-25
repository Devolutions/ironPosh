pub mod body;
pub mod fault;
pub mod header;
pub mod parsing;

use crate::cores::TagValue;
use crate::tag;
use crate::{soap::body::Body, soap::header::Header};
use ironposh_xml::mapping::{FromXml, NodeExt};

tag!(Envelope = SoapEnvelope<'a> => SoapEnvelope2003);

#[derive(Debug, Clone, typed_builder::TypedBuilder)]
pub struct SoapEnvelope<'a> {
    #[builder(default, setter(into, strip_option))]
    pub header: Option<Header<'a>>,
    #[builder(setter(into))]
    pub body: Body<'a>,
}

impl<'a> FromXml<'a> for SoapEnvelope<'a> {
    fn from_xml(node: ironposh_xml::parser::Node<'a, 'a>) -> Result<Self, ironposh_xml::XmlError> {
        // Public parse entry point: validate the root really is {SOAP}Envelope.
        // The derived value parser alone validates only children, so it would
        // accept any wrapper that happens to carry a Body.
        if !node.is_element_named(
            <EnvelopeTag as crate::cores::TagName>::NAMESPACE,
            <EnvelopeTag as crate::cores::TagName>::TAG_NAME,
        ) {
            return Err(ironposh_xml::XmlError::XmlInvalidTag {
                expected: <EnvelopeTag as crate::cores::TagName>::TAG_NAME.to_string(),
                found: node.tag_name().name().to_string(),
            });
        }
        ironposh_xml::mapping::reject_mixed_content(node)?;
        let mut header = None;
        let mut body = None;
        for child in node.children() {
            if !child.is_element() {
                continue;
            }
            if child.is_element_named(
                <Header as crate::cores::NamedTag>::NAMESPACE,
                <Header as crate::cores::NamedTag>::TAG_NAME,
            ) {
                if header.is_some() {
                    return Err(ironposh_xml::XmlError::InvalidXml(
                        "duplicate <Header> in Envelope".into(),
                    ));
                }
                header = Some(Header::from_xml(child)?);
            } else if child.is_element_named(
                <Body as crate::cores::NamedTag>::NAMESPACE,
                <Body as crate::cores::NamedTag>::TAG_NAME,
            ) {
                if body.is_some() {
                    return Err(ironposh_xml::XmlError::InvalidXml(
                        "duplicate <Body> in Envelope".into(),
                    ));
                }
                body = Some(Body::from_xml(child)?);
            }
        }
        Ok(SoapEnvelope {
            header,
            body: body.ok_or_else(|| {
                ironposh_xml::XmlError::InvalidXml("Missing body in SoapEnvelope".into())
            })?,
        })
    }
}

impl<'a> TagValue<'a> for SoapEnvelope<'a> {
    fn append_to_element(
        self,
        element: ironposh_xml::builder::Element<'a>,
    ) -> ironposh_xml::builder::Element<'a> {
        let envelope = element;

        if let Some(header) = self.header {
            envelope.add_child(header.into_element())
        } else {
            envelope
        }
        .add_child(self.body.into_element())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ironposh_xml::parser::parse;

    const S: &str = "http://www.w3.org/2003/05/soap-envelope";

    /// A non-Envelope root carrying a valid Body must be rejected, not accepted
    /// as an envelope.
    #[test]
    fn rejects_non_envelope_root_with_body() {
        let xml = format!(r#"<NotEnvelope xmlns:s="{S}"><s:Body/></NotEnvelope>"#);
        let doc = parse(&xml).unwrap();
        assert!(SoapEnvelope::from_xml(doc.root_element()).is_err());
    }

    #[test]
    fn accepts_envelope_root_regardless_of_prefix() {
        let xml = format!(r#"<x:Envelope xmlns:x="{S}"><x:Body/></x:Envelope>"#);
        let doc = parse(&xml).unwrap();
        assert!(SoapEnvelope::from_xml(doc.root_element()).is_ok());
    }
}
