use crate::cores::{
    BufferMode, ClientIP, CompressionMode, CreationXml, DataLocaleText, Encoding, IdleTimeOut,
    InputStreams, LocaleText, MaxIdleTimeOut, Name, OutputStreams, Owner, ProcessId, ProfileLoaded,
    ResourceUri, ShellId, ShellInactivity, ShellRunTime, State,
};
use crate::tag;
use ironposh_macros::{FromXml, SimpleTagValue};

tag!(Shell = ShellValue<'a> => WsmanShell);

#[derive(Debug, Clone, typed_builder::TypedBuilder, SimpleTagValue, FromXml)]
pub struct ShellValue<'a> {
    #[builder(default, setter(strip_option, into))]
    pub shell_id: Option<ShellId<'a>>,
    #[builder(default, setter(strip_option, into))]
    pub name: Option<Name<'a>>,
    #[builder(default, setter(strip_option, into))]
    pub resource_uri: Option<ResourceUri<'a>>,
    #[builder(default, setter(strip_option, into))]
    pub owner: Option<Owner<'a>>,
    #[builder(default, setter(strip_option, into))]
    pub client_ip: Option<ClientIP<'a>>,
    #[builder(default, setter(strip_option, into))]
    pub process_id: Option<ProcessId<'a>>,
    #[builder(default, setter(strip_option(fallback_suffix = "_opt"), into))]
    pub idle_time_out: Option<IdleTimeOut<'a>>,
    #[builder(default, setter(strip_option, into))]
    pub input_streams: Option<InputStreams<'a>>,
    #[builder(default, setter(strip_option, into))]
    pub output_streams: Option<OutputStreams<'a>>,
    #[builder(default, setter(strip_option, into))]
    pub max_idle_time_out: Option<MaxIdleTimeOut<'a>>,
    #[builder(default, setter(strip_option, into))]
    pub locale: Option<LocaleText<'a>>,
    #[builder(default, setter(strip_option, into))]
    pub data_locale: Option<DataLocaleText<'a>>,
    #[builder(default, setter(strip_option, into))]
    pub compression_mode: Option<CompressionMode<'a>>,
    #[builder(default, setter(strip_option, into))]
    pub profile_loaded: Option<ProfileLoaded<'a>>,
    #[builder(default, setter(strip_option, into))]
    pub encoding: Option<Encoding<'a>>,
    #[builder(default, setter(strip_option, into))]
    pub buffer_mode: Option<BufferMode<'a>>,
    #[builder(default, setter(strip_option, into))]
    pub state: Option<State<'a>>,
    #[builder(default, setter(strip_option, into))]
    pub shell_run_time: Option<ShellRunTime<'a>>,
    #[builder(default, setter(strip_option, into))]
    pub shell_inactivity: Option<ShellInactivity<'a>>,
    #[builder(default, setter(strip_option, into))]
    pub creation_xml: Option<CreationXml<'a>>,
}
