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

#[derive(Debug, Clone, typed_builder::TypedBuilder, SimpleTagValue, FromXml)]
pub struct SoapFaultReasonValue<'a> {
    #[builder(default, setter(into, strip_option))]
    pub text: Option<SoapText<'a>>,
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
