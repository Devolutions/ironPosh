use crate::traits::namespace::*;
use crate::{define_custom_tagname, define_tagname};

pub trait TagName {
    const TAG_NAME: &'static str;
    const NAMESPACE: Option<&'static str>;
}

// ==========================
// PowerShell Remoting Shell
// ==========================
define_tagname!(ShellId, Some(PWSH_NAMESPACE));
define_tagname!(Name, Some(PWSH_NAMESPACE));
define_tagname!(ResourceUri, Some(PWSH_NAMESPACE));
define_tagname!(Owner, Some(PWSH_NAMESPACE));
define_tagname!(ClientIP, Some(PWSH_NAMESPACE));
define_tagname!(ProcessId, Some(PWSH_NAMESPACE));
define_tagname!(IdleTimeOut, Some(PWSH_NAMESPACE));
define_tagname!(InputStreams, Some(PWSH_NAMESPACE));
define_tagname!(OutputStreams, Some(PWSH_NAMESPACE));
define_tagname!(MaxIdleTimeOut, Some(PWSH_NAMESPACE));
define_tagname!(Locale, Some(PWSH_NAMESPACE));
define_tagname!(DataLocale, Some(PWSH_NAMESPACE));
define_tagname!(CompressionMode, Some(PWSH_NAMESPACE));
define_tagname!(ProfileLoaded, Some(PWSH_NAMESPACE));
define_tagname!(Encoding, Some(PWSH_NAMESPACE));
define_tagname!(BufferMode, Some(PWSH_NAMESPACE));
define_tagname!(State, Some(PWSH_NAMESPACE));
define_tagname!(ShellRunTime, Some(PWSH_NAMESPACE));
define_tagname!(ShellInactivity, Some(PWSH_NAMESPACE));
define_custom_tagname!(CreationXml, "creationXml", None);

// ====================
// WS-Addressing (WSA)
// ====================
define_tagname!(Action, Some(WSA_NAMESPACE));
define_tagname!(To, Some(WSA_NAMESPACE));
define_tagname!(MessageID, Some(WSA_NAMESPACE));
define_tagname!(RelatesTo, Some(WSA_NAMESPACE));
define_tagname!(ReplyTo, Some(WSA_NAMESPACE));
define_tagname!(FaultTo, Some(WSA_NAMESPACE));
define_tagname!(From, Some(WSA_NAMESPACE));
define_tagname!(Address, Some(WSA_NAMESPACE));

// =============
// SOAP (Envelope)
// =============
define_tagname!(Envelope, Some(SOAP_NAMESPACE));
define_tagname!(Header, Some(SOAP_NAMESPACE));
define_tagname!(Body, Some(SOAP_NAMESPACE));

// ===========================
// WS-Management (WSMAN)
// ===========================
define_tagname!(Identify, Some(WSMAN_NAMESPACE));
define_tagname!(Get, Some(WSMAN_NAMESPACE));
define_tagname!(Put, Some(WSMAN_NAMESPACE));
define_tagname!(Create, Some(WSMAN_NAMESPACE));
define_tagname!(Delete, Some(WSMAN_NAMESPACE));
define_tagname!(Enumerate, Some(WSMAN_NAMESPACE));
define_tagname!(Pull, Some(WSMAN_NAMESPACE));
define_tagname!(Release, Some(WSMAN_NAMESPACE));
define_tagname!(GetStatus, Some(WSMAN_NAMESPACE));

// WSMAN Headers
define_tagname!(ResourceURI, Some(WSMAN_NAMESPACE));
define_tagname!(OperationTimeout, Some(WSMAN_NAMESPACE));
define_tagname!(MaxEnvelopeSize, Some(WSMAN_NAMESPACE));
define_tagname!(SequenceId, Some(WSMAN_NAMESPACE));
define_tagname!(OperationID, Some(WSMAN_NAMESPACE));
define_tagname!(FragmentTransfer, Some(WSMAN_NAMESPACE));
define_tagname!(SelectorSet, Some(WSMAN_NAMESPACE));
define_tagname!(SessionId, Some(WSMAN_NAMESPACE));
define_tagname!(CompressionType, Some(WSMAN_NAMESPACE));
define_tagname!(OptionSet, Some(WSMAN_NAMESPACE));
