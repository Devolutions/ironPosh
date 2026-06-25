use crate::cores::{Detail, SoapText, SoapValue};
use crate::tag;
use ironposh_macros::{FromXml, SimpleTagValue};

// SOAP Fault structures for handling SOAP error responses

tag!(Fault = SoapFaultValue<'a> => SoapEnvelope2003);
tag!(Code = SoapFaultCodeValue<'a> => SoapEnvelope2003);
tag!(Subcode = SoapFaultSubcodeValue<'a> => SoapEnvelope2003);
tag!(Reason = SoapFaultReasonValue<'a> => SoapEnvelope2003);

#[derive(Debug, Clone, typed_builder::TypedBuilder, SimpleTagValue, FromXml)]
pub struct SoapFaultValue<'a> {
    #[builder(default, setter(into, strip_option))]
    pub code: Option<Code<'a>>,
    #[builder(default, setter(into, strip_option))]
    pub reason: Option<Reason<'a>>,
    #[builder(default, setter(into, strip_option))]
    pub detail: Option<Detail<'a>>,
}

#[derive(Debug, Clone, typed_builder::TypedBuilder, SimpleTagValue, FromXml)]
pub struct SoapFaultCodeValue<'a> {
    #[builder(default, setter(into, strip_option))]
    pub value: Option<SoapValue<'a>>,
    #[builder(default, setter(into, strip_option))]
    pub subcode: Option<Subcode<'a>>,
}

#[derive(Debug, Clone, typed_builder::TypedBuilder, SimpleTagValue, FromXml)]
pub struct SoapFaultSubcodeValue<'a> {
    #[builder(default, setter(into, strip_option))]
    pub value: Option<SoapValue<'a>>,
}

#[derive(Debug, Clone, typed_builder::TypedBuilder, SimpleTagValue)]
pub struct SoapFaultReasonValue<'a> {
    #[builder(default, setter(into, strip_option))]
    pub text: Option<SoapText<'a>>,
}

impl<'a> ironposh_xml::mapping::FromXml<'a> for SoapFaultReasonValue<'a> {
    fn from_xml(node: ironposh_xml::parser::Node<'a, 'a>) -> Result<Self, ironposh_xml::XmlError> {
        use ironposh_xml::mapping::NodeExt;
        ironposh_xml::mapping::reject_mixed_content(node)?;
        // SOAP 1.2 permits several <Text xml:lang="..."> reason entries; keep the
        // first rather than rejecting a valid multilingual fault as a duplicate.
        let mut text = None;
        for child in node.children() {
            if text.is_none()
                && child.is_element_named(
                    <SoapText as crate::cores::NamedTag>::NAMESPACE,
                    <SoapText as crate::cores::NamedTag>::TAG_NAME,
                )
            {
                text = Some(<SoapText as ironposh_xml::mapping::FromXml>::from_xml(
                    child,
                )?);
            }
        }
        Ok(Self { text })
    }
}

impl SoapFaultValue<'_> {
    /// Check if this SOAP fault represents a WS-Management operation timeout.
    ///
    /// WinRM sends this fault when a Receive request times out without data.
    /// The subcode value will contain "TimedOut" (e.g. `w:TimedOut`).
    pub fn is_timeout(&self) -> bool {
        let subcode_text = self
            .code
            .as_ref()
            .and_then(|code| code.as_ref().subcode.as_ref())
            .and_then(|subcode| subcode.as_ref().value.as_ref())
            .map(|value| <&str>::from(value.as_ref()));

        matches!(subcode_text, Some(text) if text.contains("TimedOut"))
    }

    /// Check if this SOAP fault represents an invalid selector error.
    ///
    /// WinRM returns `w:InvalidSelectors` (often with WSManFault Code 2150858843)
    /// when a request references a `CommandId` that no longer exists (e.g. a
    /// pipeline was canceled or completed while we still had a Receive in flight).
    pub fn is_invalid_selectors(&self) -> bool {
        let subcode_text = self
            .code
            .as_ref()
            .and_then(|code| code.as_ref().subcode.as_ref())
            .and_then(|subcode| subcode.as_ref().value.as_ref())
            .map(|value| <&str>::from(value.as_ref()));

        matches!(subcode_text, Some(text) if text.contains("InvalidSelectors"))
    }

    /// Get the human-readable reason text from the fault, if available.
    pub fn reason_text(&self) -> Option<&str> {
        self.reason
            .as_ref()
            .and_then(|r| r.as_ref().text.as_ref())
            .map(|t| <&str>::from(t.as_ref()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ironposh_xml::mapping::FromXml as _;
    use ironposh_xml::parser::parse;

    const S: &str = "http://www.w3.org/2003/05/soap-envelope";

    #[test]
    fn reason_accepts_multiple_text_entries() {
        let xml = format!(
            r#"<s:Reason xmlns:s="{S}"><s:Text xml:lang="en">English</s:Text><s:Text xml:lang="fr">French</s:Text></s:Reason>"#
        );
        let doc = parse(&xml).unwrap();
        let reason =
            Reason::from_xml(doc.root_element()).expect("multilingual reason should parse");
        assert!(reason.as_ref().text.is_some());
    }
}
