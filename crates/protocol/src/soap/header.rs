use crate::{cores::*, ws_addressing::AddressValue, ws_management::OptionSetValue};

#[derive(
    Debug,
    Clone,
    typed_builder::TypedBuilder,
    protocol_macros::SimpleTagValue,
    protocol_macros::SimpleXmlDeserialize,
)]
pub struct SoapHeaders<'a> {
    /// WS-Addressing headers
    #[builder(default, setter(into, strip_option))]
    pub to: Option<Tag<'a, Text<'a>, To>>,
    #[builder(default, setter(into, strip_option))]
    pub action: Option<Tag<'a, Text<'a>, Action>>,
    #[builder(default, setter(into, strip_option))]
    pub reply_to: Option<Tag<'a, AddressValue<'a>, ReplyTo>>,
    #[builder(default, setter(into, strip_option))]
    pub message_id: Option<Tag<'a, Text<'a>, MessageID>>,
    #[builder(default, setter(into, strip_option))]
    pub relates_to: Option<Tag<'a, Text<'a>, RelatesTo>>,

    /// WS-Management headers
    #[builder(default, setter(into, strip_option))]
    pub resource_uri: Option<Tag<'a, Text<'a>, ResourceURI>>,
    #[builder(default, setter(into, strip_option))]
    pub max_envelope_size: Option<Tag<'a, Text<'a>, MaxEnvelopeSize>>,
    #[builder(default, setter(into, strip_option))]
    pub locale: Option<Tag<'a, Text<'a>, Locale>>,
    #[builder(default, setter(into, strip_option))]
    pub data_locale: Option<Tag<'a, Text<'a>, DataLocale>>,
    #[builder(default, setter(into, strip_option))]
    pub session_id: Option<Tag<'a, Text<'a>, SessionId>>,
    #[builder(default, setter(into, strip_option))]
    pub operation_id: Option<Tag<'a, Text<'a>, OperationID>>,
    #[builder(default, setter(into, strip_option))]
    pub sequence_id: Option<Tag<'a, Text<'a>, SequenceId>>,
    #[builder(default, setter(into, strip_option))]
    pub option_set: Option<Tag<'a, OptionSetValue<'a>, OptionSet>>,
    #[builder(default, setter(into, strip_option))]
    pub operation_timeout: Option<Tag<'a, Text<'a>, OperationTimeout>>,
    #[builder(default, setter(into, strip_option))]
    pub compression_type: Option<Tag<'a, Text<'a>, CompressionType>>,
}
