use crate::cores::*;
use ironposh_macros::{SimpleTagValue, SimpleXmlDeserialize};

// SOAP Fault structures for handling SOAP error responses

#[derive(Debug, Clone, typed_builder::TypedBuilder, SimpleTagValue, SimpleXmlDeserialize)]
pub struct SoapFaultValue<'a> {
    #[builder(default, setter(into, strip_option))]
    pub code: Option<Tag<'a, SoapFaultCodeValue<'a>, Code>>,
    #[builder(default, setter(into, strip_option))]
    pub reason: Option<Tag<'a, SoapFaultReasonValue<'a>, Reason>>,
    #[builder(default, setter(into, strip_option))]
    pub detail: Option<Tag<'a, ReadOnlyUnParsed<'a>, Detail>>,
}

#[derive(Debug, Clone, typed_builder::TypedBuilder, SimpleTagValue, SimpleXmlDeserialize)]
pub struct SoapFaultCodeValue<'a> {
    #[builder(default, setter(into, strip_option))]
    pub value: Option<Tag<'a, Text<'a>, SoapValue>>,
    #[builder(default, setter(into, strip_option))]
    pub subcode: Option<Tag<'a, SoapFaultSubcodeValue<'a>, Subcode>>,
}

#[derive(Debug, Clone, typed_builder::TypedBuilder, SimpleTagValue, SimpleXmlDeserialize)]
pub struct SoapFaultSubcodeValue<'a> {
    #[builder(default, setter(into, strip_option))]
    pub value: Option<Tag<'a, Text<'a>, SoapValue>>,
}

#[derive(Debug, Clone, typed_builder::TypedBuilder, SimpleTagValue, SimpleXmlDeserialize)]
pub struct SoapFaultReasonValue<'a> {
    #[builder(default, setter(into, strip_option))]
    pub text: Option<Tag<'a, Text<'a>, SoapText>>,
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

    /// Get the human-readable reason text from the fault, if available.
    pub fn reason_text(&self) -> Option<&str> {
        self.reason
            .as_ref()
            .and_then(|r| r.as_ref().text.as_ref())
            .map(|t| <&str>::from(t.as_ref()))
    }
}
