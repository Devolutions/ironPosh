use crate::cores::{
    Action, CompressionType, DataLocaleEmpty, LocaleEmpty, MaxEnvelopeSize, MessageID, OperationID,
    OperationTimeout, RelatesTo, ResourceURI, SequenceId, SessionId, To,
};
use crate::tag;
use crate::ws_addressing::ReplyTo;
use crate::ws_management::{OptionSet, SelectorSet};

tag!(Header = SoapHeaders<'a> => SoapEnvelope2003);

#[derive(
    Debug,
    Clone,
    typed_builder::TypedBuilder,
    ironposh_macros::SimpleTagValue,
    ironposh_macros::FromXml,
)]
pub struct SoapHeaders<'a> {
    /// WS-Addressing headers
    #[builder(default, setter(into, strip_option))]
    pub to: Option<To<'a>>,
    #[builder(default, setter(into, strip_option))]
    pub action: Option<Action<'a>>,
    #[builder(default, setter(into, strip_option))]
    pub reply_to: Option<ReplyTo<'a>>,
    #[builder(default, setter(into, strip_option))]
    pub message_id: Option<MessageID<'a>>,
    #[builder(default, setter(into, strip_option))]
    pub relates_to: Option<RelatesTo<'a>>,

    /// WS-Management headers
    #[builder(default, setter(into, strip_option))]
    pub resource_uri: Option<ResourceURI<'a>>,
    #[builder(default, setter(into, strip_option))]
    pub max_envelope_size: Option<MaxEnvelopeSize<'a>>,
    #[builder(default, setter(into, strip_option))]
    pub locale: Option<LocaleEmpty<'a>>,
    #[builder(default, setter(into, strip_option))]
    pub data_locale: Option<DataLocaleEmpty<'a>>,
    #[builder(default, setter(into, strip_option))]
    pub session_id: Option<SessionId<'a>>,
    #[builder(default, setter(into, strip_option))]
    pub operation_id: Option<OperationID<'a>>,
    #[builder(default, setter(into, strip_option))]
    pub sequence_id: Option<SequenceId<'a>>,
    #[builder(default, setter(into, strip_option(fallback_suffix = "_opt")))]
    pub option_set: Option<OptionSet<'a>>,
    #[builder(default, setter(into, strip_option(fallback_suffix = "_opt")))]
    pub selector_set: Option<SelectorSet<'a>>,
    #[builder(default, setter(into, strip_option))]
    pub operation_timeout: Option<OperationTimeout<'a>>,
    #[builder(default, setter(into, strip_option))]
    pub compression_type: Option<CompressionType<'a>>,
}
