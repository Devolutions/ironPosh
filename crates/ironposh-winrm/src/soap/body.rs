use ironposh_macros::{FromXml, SimpleTagValue};

use crate::tag;
use crate::{
    cores::{
        CommandResponse, Create, Delete, DisconnectResponse, Enumerate, Get, Identify, Put,
        Reconnect, ReconnectResponse, Signal, SignalResponse,
    },
    rsp::{
        commandline::CommandLine,
        connect::{Connect, ConnectResponse},
        disconnect::Disconnect,
        receive::{Receive, ReceiveResponse},
        send::Send,
        shell_value::Shell,
    },
    soap::fault::Fault,
    ws_management::body::ResourceCreated,
};

tag!(Body = SoapBody<'a> => SoapEnvelope2003);

#[derive(Debug, Clone, typed_builder::TypedBuilder, SimpleTagValue, FromXml)]
pub struct SoapBody<'a> {
    /// WS-Management operations
    #[builder(default, setter(into, strip_option))]
    pub identify: Option<Identify<'a>>,
    #[builder(default, setter(into, strip_option))]
    pub get: Option<Get<'a>>,
    #[builder(default, setter(into, strip_option))]
    pub put: Option<Put<'a>>,
    #[builder(default, setter(into, strip_option))]
    pub create: Option<Create<'a>>,
    #[builder(default, setter(into, strip_option))]
    pub delete: Option<Delete<'a>>,
    #[builder(default, setter(into, strip_option))]
    pub enumerate: Option<Enumerate<'a>>,

    /// WS-Transfer operations
    #[builder(default, setter(into, strip_option))]
    pub resource_created: Option<ResourceCreated<'a>>,

    /// PowerShell Remoting operations
    #[builder(default, setter(into, strip_option))]
    pub shell: Option<Shell<'a>>,
    #[builder(default, setter(into, strip_option))]
    pub command_line: Option<CommandLine<'a>>,
    #[builder(default, setter(into, strip_option))]
    pub receive: Option<Receive<'a>>,
    #[builder(default, setter(into, strip_option))]
    pub receive_response: Option<ReceiveResponse<'a>>,
    #[builder(default, setter(into, strip_option))]
    pub command_response: Option<CommandResponse<'a>>,
    #[builder(default, setter(into, strip_option))]
    pub send: Option<Send<'a>>,
    #[builder(default, setter(into, strip_option))]
    pub signal: Option<Signal<'a>>,
    #[builder(default, setter(into, strip_option))]
    pub signal_response: Option<SignalResponse<'a>>,
    #[builder(default, setter(into, strip_option))]
    pub disconnect: Option<Disconnect<'a>>,
    #[builder(default, setter(into, strip_option))]
    pub disconnect_response: Option<DisconnectResponse<'a>>,
    #[builder(default, setter(into, strip_option))]
    pub reconnect: Option<Reconnect<'a>>,
    #[builder(default, setter(into, strip_option))]
    pub reconnect_response: Option<ReconnectResponse<'a>>,
    #[builder(default, setter(into, strip_option))]
    pub connect: Option<Connect<'a>>,
    #[builder(default, setter(into, strip_option))]
    pub connect_response: Option<ConnectResponse<'a>>,

    /// SOAP fault handling
    #[builder(default, setter(into, strip_option))]
    pub fault: Option<Fault<'a>>,
}
