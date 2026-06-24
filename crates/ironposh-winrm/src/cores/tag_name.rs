use crate::cores::tag_value::{Empty, I32, ReadOnlyUnParsed, Text, Time, U32, WsUuid};
use crate::tag;

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

// Leaf-valued tags live here (their value types are all in `cores`). Tags whose
// value is a domain struct (Shell = ShellValue, Body = SoapBody, …) are defined
// with `tag!` next to that struct instead — see the rsp/soap/ws_* modules.

// ============================================================
// PowerShell Remoting Shell (rsp namespace)
// ============================================================
tag!(ShellId = Text<'a> => WsmanShell);
tag!(Name = Text<'a> => WsmanShell);
tag!(ResourceUri = Text<'a> => WsmanShell);
tag!(Owner = Text<'a> => WsmanShell);
tag!(ClientIP = Text<'a> => WsmanShell);
tag!(ProcessId = Text<'a> => WsmanShell);
tag!(IdleTimeOut = Time => WsmanShell);
tag!(InputStreams = Text<'a> => WsmanShell);
tag!(OutputStreams = Text<'a> => WsmanShell);
tag!(MaxIdleTimeOut = Text<'a> => WsmanShell);
tag!(CompressionMode = Text<'a> => WsmanShell);
tag!(ProfileLoaded = Text<'a> => WsmanShell);
tag!(Encoding = Text<'a> => WsmanShell);
tag!(BufferMode = Text<'a> => WsmanShell);
tag!(State = Text<'a> => WsmanShell);
tag!(ShellRunTime = Text<'a> => WsmanShell);
tag!(ShellInactivity = Text<'a> => WsmanShell);
tag!(CompressionType = Text<'a> => WsmanShell);
tag!(DesiredStream = Text<'a> => WsmanShell);
tag!(Stream = Text<'a> => WsmanShell);
tag!(ExitCode = I32 => WsmanShell);
tag!(CommandId = WsUuid => WsmanShell);
tag!(CommandResponse = CommandId<'a> => WsmanShell); // wraps a single CommandId child
tag!(Command = Text<'a> => WsmanShell);
tag!(Arguments = Text<'a> => WsmanShell);
tag!(DisconnectResponse = Empty => WsmanShell);
tag!(Reconnect = Empty => WsmanShell);
tag!(ReconnectResponse = Empty => WsmanShell);
tag!(SignalCode = "Code": Text<'a> => WsmanShell);
tag!(Signal = SignalCode<'a> => WsmanShell); // wraps a single Code child
tag!(SignalResponse = Empty => WsmanShell);

tag!(CreationXml = "creationXml": Text<'a> => PowerShellRemoting);
tag!(ConnectXml = "connectXml": Text<'a> => PowerShellRemoting);
tag!(ConnectResponseXml = "connectResponseXml": Text<'a> => PowerShellRemoting);

// ============================================================
// WS-Addressing (a namespace)
// ============================================================
tag!(Action = Text<'a> => WsAddressing2004);
tag!(To = Text<'a> => WsAddressing2004);
tag!(MessageID = WsUuid => WsAddressing2004);
tag!(RelatesTo = WsUuid => WsAddressing2004);
tag!(Address = Text<'a> => WsAddressing2004);

// ============================================================
// SOAP (s namespace)
// ============================================================
tag!(Detail = ReadOnlyUnParsed<'a> => SoapEnvelope2003);
tag!(SoapValue = "Value": Text<'a> => SoapEnvelope2003);
tag!(SoapText = "Text": Text<'a> => SoapEnvelope2003);

// ============================================================
// WS-Management DMTF (w namespace)
// ============================================================
tag!(Identify = Empty => DmtfWsmanSchema);
tag!(Get = Text<'a> => DmtfWsmanSchema);
tag!(Put = Text<'a> => DmtfWsmanSchema);
tag!(Delete = Text<'a> => DmtfWsmanSchema);
tag!(Enumerate = ReadOnlyUnParsed<'a> => DmtfWsmanSchema);
tag!(ResourceURI = Text<'a> => DmtfWsmanSchema);
tag!(OperationTimeout = Time => DmtfWsmanSchema);
tag!(MaxEnvelopeSize = U32 => DmtfWsmanSchema);
tag!(Selector = Text<'a> => DmtfWsmanSchema);
tag!(OptionTagName = "Option": Empty => DmtfWsmanSchema);
tag!(LocaleEmpty = "Locale": Empty => DmtfWsmanSchema);
tag!(LocaleText = "Locale": Text<'a> => DmtfWsmanSchema);

// ============================================================
// WS-Transfer (x namespace)
// ============================================================
tag!(Create = Text<'a> => WsTransfer2004);

// ============================================================
// Microsoft WS-Management (p namespace)
// ============================================================
tag!(SequenceId = Text<'a> => MsWsmanSchema);
tag!(OperationID = WsUuid => MsWsmanSchema);
tag!(SessionId = WsUuid => MsWsmanSchema);
tag!(DataLocaleEmpty = "DataLocale": Empty => MsWsmanSchema);
tag!(DataLocaleText = "DataLocale": Text<'a> => MsWsmanSchema);
