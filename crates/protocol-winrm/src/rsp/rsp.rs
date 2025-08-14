use crate::{cores::{
    tag_name::{
        BufferMode, ClientIP, CompressionMode, CreationXml, DataLocale, Encoding, IdleTimeOut,
        InputStreams, Locale, MaxIdleTimeOut, Name, OutputStreams, Owner, ProcessId, ProfileLoaded,
        ResourceUri, ShellId, ShellInactivity, ShellRunTime, State, TagName,
    }, CommandLine, Tag, Text, Time
}, rsp::commandline::CommandLineValue};
use protocol_macros::{SimpleTagValue, SimpleXmlDeserialize};

// The XmlTagContainer derive macro generates:
// - TagValue implementation
// - ShellValueVisitor struct
// - XmlVisitor implementation for ShellValueVisitor
// - XmlDeserialize implementation
#[derive(Debug, Clone, typed_builder::TypedBuilder, SimpleTagValue, SimpleXmlDeserialize)]
pub struct ShellValue<'a> {
    #[builder(default, setter(strip_option, into))]
    pub shell_id: Option<Tag<'a, Text<'a>, ShellId>>,
    #[builder(default, setter(strip_option, into))]
    pub name: Option<Tag<'a, Text<'a>, Name>>,
    #[builder(default, setter(strip_option, into))]
    pub resource_uri: Option<Tag<'a, Text<'a>, ResourceUri>>,
    #[builder(default, setter(strip_option, into))]
    pub owner: Option<Tag<'a, Text<'a>, Owner>>,
    #[builder(default, setter(strip_option, into))]
    pub client_ip: Option<Tag<'a, Text<'a>, ClientIP>>,
    #[builder(default, setter(strip_option, into))]
    pub process_id: Option<Tag<'a, Text<'a>, ProcessId>>,
    #[builder(default, setter(strip_option(fallback_suffix = "_opt"), into))]
    pub idle_time_out: Option<Tag<'a, Time, IdleTimeOut>>,
    #[builder(default, setter(strip_option, into))]
    pub input_streams: Option<Tag<'a, Text<'a>, InputStreams>>,
    #[builder(default, setter(strip_option, into))]
    pub output_streams: Option<Tag<'a, Text<'a>, OutputStreams>>,
    #[builder(default, setter(strip_option, into))]
    pub max_idle_time_out: Option<Tag<'a, Text<'a>, MaxIdleTimeOut>>,
    #[builder(default, setter(strip_option, into))]
    pub locale: Option<Tag<'a, Text<'a>, Locale>>,
    #[builder(default, setter(strip_option, into))]
    pub data_locale: Option<Tag<'a, Text<'a>, DataLocale>>,
    #[builder(default, setter(strip_option, into))]
    pub compression_mode: Option<Tag<'a, Text<'a>, CompressionMode>>,
    #[builder(default, setter(strip_option, into))]
    pub profile_loaded: Option<Tag<'a, Text<'a>, ProfileLoaded>>,
    #[builder(default, setter(strip_option, into))]
    pub encoding: Option<Tag<'a, Text<'a>, Encoding>>,
    #[builder(default, setter(strip_option, into))]
    pub buffer_mode: Option<Tag<'a, Text<'a>, BufferMode>>,
    #[builder(default, setter(strip_option, into))]
    pub state: Option<Tag<'a, Text<'a>, State>>,
    #[builder(default, setter(strip_option, into))]
    pub shell_run_time: Option<Tag<'a, Text<'a>, ShellRunTime>>,
    #[builder(default, setter(strip_option, into))]
    pub shell_inactivity: Option<Tag<'a, Text<'a>, ShellInactivity>>,
    #[builder(default, setter(strip_option, into))]
    pub creation_xml: Option<Tag<'a, Text<'a>, CreationXml>>,
}
