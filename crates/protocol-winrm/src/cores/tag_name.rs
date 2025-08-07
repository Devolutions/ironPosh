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

define_custom_tagname!(CreationXml, "creationXml", None);

// PowerShell Remoting Operations (rsp namespace)
define_tagname!(Shell, Some(Namespace::WsmanShell.uri()));
define_tagname!(Command, Some(Namespace::WsmanShell.uri()));
define_tagname!(Receive, Some(Namespace::WsmanShell.uri()));
define_tagname!(Send, Some(Namespace::WsmanShell.uri()));
define_tagname!(Signal, Some(Namespace::WsmanShell.uri()));

// ====================
// WS-Addressing (a namespace)
// ====================
define_tagname!(Action, Some(Namespace::WsAddressing2004.uri()));
define_tagname!(To, Some(Namespace::WsAddressing2004.uri()));
define_tagname!(MessageID, Some(Namespace::WsAddressing2004.uri()));
define_tagname!(RelatesTo, Some(Namespace::WsAddressing2004.uri()));
define_tagname!(ReplyTo, Some(Namespace::WsAddressing2004.uri()));
define_tagname!(FaultTo, Some(Namespace::WsAddressing2004.uri()));
define_tagname!(From, Some(Namespace::WsAddressing2004.uri()));
define_tagname!(Address, Some(Namespace::WsAddressing2004.uri()));
define_tagname!(ReferenceParameters, Some(Namespace::WsAddressing2004.uri()));

// =============
// SOAP (s namespace)
// =============
define_tagname!(Envelope, Some(Namespace::SoapEnvelope2003.uri()));
define_tagname!(Header, Some(Namespace::SoapEnvelope2003.uri()));
define_tagname!(Body, Some(Namespace::SoapEnvelope2003.uri()));

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
define_tagname!(FragmentTransfer, Some(Namespace::DmtfWsmanSchema.uri()));
define_tagname!(SelectorSet, Some(Namespace::DmtfWsmanSchema.uri()));
define_tagname!(OptionSet, Some(Namespace::DmtfWsmanSchema.uri()));
define_tagname!(Locale, Some(Namespace::DmtfWsmanSchema.uri()));
define_custom_tagname!(
    OptionTagName,
    "Option",
    Some(Namespace::DmtfWsmanSchema.uri())
);
define_custom_tagname!(
    SelectorTagName,
    "Selector",
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

// PowerShell Remoting Protocol;
define_tagname!(Obj, None);
define_tagname!(MS, None);
define_tagname!(Version, None);
define_tagname!(BA, None);

// PowerShell Serialization Format
define_tagname!(I32, None); // 32-bit integer
define_tagname!(TN, None); // Type Name
define_tagname!(T, None); // Type
define_custom_tagname!(ToString, "ToString", None); // ToString representation
define_tagname!(DCT, None); // Dictionary
define_tagname!(En, None); // Dictionary Entry
define_tagname!(Key, None); // Dictionary Key
define_tagname!(Value, None); // Dictionary Value
define_tagname!(Nil, None); // Null Value
define_tagname!(B, None); // Boolean
define_tagname!(S, None); // String

// PowerShell InitRunspacepool Message Tags
define_tagname!(MinRunspaces, None); // Minimum number of runspaces
define_tagname!(MaxRunspaces, None); // Maximum number of runspaces
define_tagname!(PSThreadOptions, None); // PowerShell thread options
define_tagname!(ApartmentState, None); // Apartment state for runspace
define_tagname!(HostInfo, None); // Host information
define_tagname!(ApplicationArguments, None); // Application arguments
