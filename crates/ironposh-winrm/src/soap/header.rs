use crate::{
    cores::*,
    ws_addressing::AddressValue,
    ws_management::{OptionSetValue, SelectorSetValue},
};

#[derive(
    Debug,
    Clone,
    typed_builder::TypedBuilder,
    ironposh_macros::SimpleTagValue,
    ironposh_macros::SimpleXmlDeserialize,
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
    pub message_id: Option<Tag<'a, WsUuid, MessageID>>,
    #[builder(default, setter(into, strip_option))]
    pub relates_to: Option<Tag<'a, Text<'a>, RelatesTo>>,

    /// WS-Management headers
    #[builder(default, setter(into, strip_option))]
    pub resource_uri: Option<Tag<'a, Text<'a>, ResourceURI>>,
    #[builder(default, setter(into, strip_option))]
    pub max_envelope_size: Option<Tag<'a, U32, MaxEnvelopeSize>>,
    #[builder(default, setter(into, strip_option))]
    pub locale: Option<Tag<'a, Empty, Locale>>,
    #[builder(default, setter(into, strip_option))]
    pub data_locale: Option<Tag<'a, Empty, DataLocale>>,
    #[builder(default, setter(into, strip_option))]
    pub session_id: Option<Tag<'a, WsUuid, SessionId>>,
    #[builder(default, setter(into, strip_option))]
    pub operation_id: Option<Tag<'a, WsUuid, OperationID>>,
    #[builder(default, setter(into, strip_option))]
    pub sequence_id: Option<Tag<'a, Text<'a>, SequenceId>>,
    #[builder(default, setter(into, strip_option(fallback_suffix = "_opt")))]
    pub option_set: Option<Tag<'a, OptionSetValue, OptionSet>>,
    #[builder(default, setter(into, strip_option(fallback_suffix = "_opt")))]
    pub selector_set: Option<Tag<'a, SelectorSetValue, SelectorSet>>,
    #[builder(default, setter(into, strip_option))]
    pub operation_timeout: Option<Tag<'a, Time, OperationTimeout>>,
    #[builder(default, setter(into, strip_option))]
    pub compression_type: Option<Tag<'a, Text<'a>, CompressionType>>,
}
