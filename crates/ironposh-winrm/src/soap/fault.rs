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
