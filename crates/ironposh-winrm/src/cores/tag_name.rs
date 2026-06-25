use crate::cores::namespace::*;
use crate::{define_custom_tagname, define_tagname};

pub trait TagName {
    const TAG_NAME: &'static str;
    const NAMESPACE: Option<&'static str>;

    fn tag_name(&self) -> &'static str {
        Self::TAG_NAME
    }

    fn namespace(&self) -> Option<&'static str> {
        Self::NAMESPACE
    }
}

// ==========================
// PowerShell Remoting Shell (rsp namespace)
// ==========================
define_tagname!(ShellId, Some(Namespace::WsmanShell.uri()));
define_tagname!(Name, Some(Namespace::WsmanShell.uri()));
define_tagname!(ResourceUri, Some(Namespace::WsmanShell.uri()));
define_tagname!(Owner, Some(Namespace::WsmanShell.uri()));
define_tagname!(ClientIP, Some(Namespace::WsmanShell.uri()));
define_tagname!(ProcessId, Some(Namespace::WsmanShell.uri()));
define_tagname!(IdleTimeOut, Some(Namespace::WsmanShell.uri()));
define_tagname!(InputStreams, Some(Namespace::WsmanShell.uri()));
define_tagname!(OutputStreams, Some(Namespace::WsmanShell.uri()));
define_tagname!(MaxIdleTimeOut, Some(Namespace::WsmanShell.uri()));
define_tagname!(CompressionMode, Some(Namespace::WsmanShell.uri()));
define_tagname!(ProfileLoaded, Some(Namespace::WsmanShell.uri()));
define_tagname!(Encoding, Some(Namespace::WsmanShell.uri()));
define_tagname!(BufferMode, Some(Namespace::WsmanShell.uri()));
define_tagname!(State, Some(Namespace::WsmanShell.uri()));
define_tagname!(ShellRunTime, Some(Namespace::WsmanShell.uri()));
define_tagname!(ShellInactivity, Some(Namespace::WsmanShell.uri()));
define_tagname!(CompressionType, Some(Namespace::WsmanShell.uri()));
define_tagname!(DesiredStream, Some(Namespace::WsmanShell.uri()));

define_custom_tagname!(
    CreationXml,
    "creationXml",
    Some(Namespace::PowerShellRemoting.uri())
);

define_tagname!(CommandLine, Some(Namespace::WsmanShell.uri()));
define_tagname!(Shell, Some(Namespace::WsmanShell.uri()));
define_tagname!(Command, Some(Namespace::WsmanShell.uri()));
define_tagname!(Receive, Some(Namespace::WsmanShell.uri()));
define_tagname!(ReceiveResponse, Some(Namespace::WsmanShell.uri()));
define_tagname!(CommandResponse, Some(Namespace::WsmanShell.uri()));
define_tagname!(CommandId, Some(Namespace::WsmanShell.uri()));
define_tagname!(Stream, Some(Namespace::WsmanShell.uri()));
define_tagname!(CommandState, Some(Namespace::WsmanShell.uri()));
define_tagname!(ExitCode, Some(Namespace::WsmanShell.uri()));
define_tagname!(Send, Some(Namespace::WsmanShell.uri()));
define_tagname!(Disconnect, Some(Namespace::WsmanShell.uri()));
define_tagname!(DisconnectResponse, Some(Namespace::WsmanShell.uri()));
define_tagname!(Reconnect, Some(Namespace::WsmanShell.uri()));
define_tagname!(ReconnectResponse, Some(Namespace::WsmanShell.uri()));
define_tagname!(Connect, Some(Namespace::WsmanShell.uri()));
define_tagname!(ConnectResponse, Some(Namespace::WsmanShell.uri()));
define_custom_tagname!(
    ConnectXml,
    "connectXml",
    Some(Namespace::PowerShellRemoting.uri())
);
define_custom_tagname!(
    ConnectResponseXml,
    "connectResponseXml",
    Some(Namespace::PowerShellRemoting.uri())
);
define_tagname!(Signal, Some(Namespace::WsmanShell.uri()));
define_tagname!(SignalResponse, Some(Namespace::WsmanShell.uri()));
define_custom_tagname!(SignalCode, "Code", Some(Namespace::WsmanShell.uri()));
define_tagname!(Arguments, Some(Namespace::WsmanShell.uri()));

// ====================
// WS-Addressing (a namespace)
// ====================
define_tagname!(Action, Some(Namespace::WsAddressing2004.uri()));
define_tagname!(To, Some(Namespace::WsAddressing2004.uri()));
define_tagname!(MessageID, Some(Namespace::WsAddressing2004.uri()));
define_tagname!(RelatesTo, Some(Namespace::WsAddressing2004.uri()));
define_tagname!(ReplyTo, Some(Namespace::WsAddressing2004.uri()));
define_tagname!(Address, Some(Namespace::WsAddressing2004.uri()));
define_tagname!(ReferenceParameters, Some(Namespace::WsAddressing2004.uri()));

// =============
// SOAP (s namespace)
// =============
define_tagname!(Envelope, Some(Namespace::SoapEnvelope2003.uri()));
define_tagname!(Header, Some(Namespace::SoapEnvelope2003.uri()));
define_tagname!(Body, Some(Namespace::SoapEnvelope2003.uri()));

// SOAP Fault elements
define_tagname!(Fault, Some(Namespace::SoapEnvelope2003.uri()));
define_tagname!(Code, Some(Namespace::SoapEnvelope2003.uri()));
define_tagname!(Reason, Some(Namespace::SoapEnvelope2003.uri()));
define_tagname!(Detail, Some(Namespace::SoapEnvelope2003.uri()));
define_tagname!(Subcode, Some(Namespace::SoapEnvelope2003.uri()));
define_custom_tagname!(SoapValue, "Value", Some(Namespace::SoapEnvelope2003.uri()));
define_custom_tagname!(SoapText, "Text", Some(Namespace::SoapEnvelope2003.uri()));

// ===============================
// WS-Management DMTF (w namespace)
// ===============================
define_tagname!(Identify, Some(Namespace::DmtfWsmanSchema.uri()));
define_tagname!(Get, Some(Namespace::DmtfWsmanSchema.uri()));
define_tagname!(Put, Some(Namespace::DmtfWsmanSchema.uri()));
define_tagname!(Delete, Some(Namespace::DmtfWsmanSchema.uri()));
define_tagname!(Enumerate, Some(Namespace::DmtfWsmanSchema.uri()));
define_tagname!(Pull, Some(Namespace::DmtfWsmanSchema.uri()));
define_tagname!(Release, Some(Namespace::DmtfWsmanSchema.uri()));
define_tagname!(GetStatus, Some(Namespace::DmtfWsmanSchema.uri()));

// WS-Management DMTF Headers (w namespace)
define_tagname!(ResourceURI, Some(Namespace::DmtfWsmanSchema.uri()));
define_tagname!(OperationTimeout, Some(Namespace::DmtfWsmanSchema.uri()));
define_tagname!(MaxEnvelopeSize, Some(Namespace::DmtfWsmanSchema.uri()));
define_tagname!(SelectorSet, Some(Namespace::DmtfWsmanSchema.uri()));
define_tagname!(OptionSet, Some(Namespace::DmtfWsmanSchema.uri()));
define_tagname!(Locale, Some(Namespace::DmtfWsmanSchema.uri()));
define_tagname!(Selector, Some(Namespace::DmtfWsmanSchema.uri()));
define_custom_tagname!(
    OptionTagName,
    "Option",
    Some(Namespace::DmtfWsmanSchema.uri())
);

// ===================================
// WS-Transfer (x namespace)
// ===================================
define_tagname!(Create, Some(Namespace::WsTransfer2004.uri()));

define_tagname!(ResourceCreated, Some(Namespace::WsTransfer2004.uri()));

// ====================================
// Microsoft WS-Management (p namespace)
// ====================================
define_tagname!(SequenceId, Some(Namespace::MsWsmanSchema.uri()));
define_tagname!(OperationID, Some(Namespace::MsWsmanSchema.uri()));
define_tagname!(SessionId, Some(Namespace::MsWsmanSchema.uri()));
define_tagname!(DataLocale, Some(Namespace::MsWsmanSchema.uri()));
