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
define_tagname!(ShellId, Some(WSMAN_SHELL_NAMESPACE_URI));
define_tagname!(Name, Some(WSMAN_SHELL_NAMESPACE_URI));
define_tagname!(ResourceUri, Some(WSMAN_SHELL_NAMESPACE_URI));
define_tagname!(Owner, Some(WSMAN_SHELL_NAMESPACE_URI));
define_tagname!(ClientIP, Some(WSMAN_SHELL_NAMESPACE_URI));
define_tagname!(ProcessId, Some(WSMAN_SHELL_NAMESPACE_URI));
define_tagname!(IdleTimeOut, Some(WSMAN_SHELL_NAMESPACE_URI));
define_tagname!(InputStreams, Some(WSMAN_SHELL_NAMESPACE_URI));
define_tagname!(OutputStreams, Some(WSMAN_SHELL_NAMESPACE_URI));
define_tagname!(MaxIdleTimeOut, Some(WSMAN_SHELL_NAMESPACE_URI));
define_tagname!(CompressionMode, Some(WSMAN_SHELL_NAMESPACE_URI));
define_tagname!(ProfileLoaded, Some(WSMAN_SHELL_NAMESPACE_URI));
define_tagname!(Encoding, Some(WSMAN_SHELL_NAMESPACE_URI));
define_tagname!(BufferMode, Some(WSMAN_SHELL_NAMESPACE_URI));
define_tagname!(State, Some(WSMAN_SHELL_NAMESPACE_URI));
define_tagname!(ShellRunTime, Some(WSMAN_SHELL_NAMESPACE_URI));
define_tagname!(ShellInactivity, Some(WSMAN_SHELL_NAMESPACE_URI));
define_tagname!(CompressionType, Some(WSMAN_SHELL_NAMESPACE_URI));
define_custom_tagname!(CreationXml, "creationXml", None);

// PowerShell Remoting Operations (rsp namespace)
define_tagname!(Shell, Some(WSMAN_SHELL_NAMESPACE_URI));
define_tagname!(Command, Some(WSMAN_SHELL_NAMESPACE_URI));
define_tagname!(Receive, Some(WSMAN_SHELL_NAMESPACE_URI));
define_tagname!(Send, Some(WSMAN_SHELL_NAMESPACE_URI));
define_tagname!(Signal, Some(WSMAN_SHELL_NAMESPACE_URI));

// ====================
// WS-Addressing (a namespace)
// ====================
define_tagname!(Action, Some(WS_ADDRESSING_NAMESPACE_URI));
define_tagname!(To, Some(WS_ADDRESSING_NAMESPACE_URI));
define_tagname!(MessageID, Some(WS_ADDRESSING_NAMESPACE_URI));
define_tagname!(RelatesTo, Some(WS_ADDRESSING_NAMESPACE_URI));
define_tagname!(ReplyTo, Some(WS_ADDRESSING_NAMESPACE_URI));
define_tagname!(FaultTo, Some(WS_ADDRESSING_NAMESPACE_URI));
define_tagname!(From, Some(WS_ADDRESSING_NAMESPACE_URI));
define_tagname!(Address, Some(WS_ADDRESSING_NAMESPACE_URI));
define_tagname!(ReferenceParameters, Some(WS_ADDRESSING_NAMESPACE_URI));

// =============
// SOAP (s namespace)
// =============
define_tagname!(Envelope, Some(SOAP_ENVELOPE_NAMESPACE_URI));
define_tagname!(Header, Some(SOAP_ENVELOPE_NAMESPACE_URI));
define_tagname!(Body, Some(SOAP_ENVELOPE_NAMESPACE_URI));

// ===============================
// WS-Management DMTF (w namespace)
// ===============================
define_tagname!(Identify, Some(DMTF_WSMAN_SCHEMA_NAMESPACE_URI));
define_tagname!(Get, Some(DMTF_WSMAN_SCHEMA_NAMESPACE_URI));
define_tagname!(Put, Some(DMTF_WSMAN_SCHEMA_NAMESPACE_URI));
define_tagname!(Delete, Some(DMTF_WSMAN_SCHEMA_NAMESPACE_URI));
define_tagname!(Enumerate, Some(DMTF_WSMAN_SCHEMA_NAMESPACE_URI));
define_tagname!(Pull, Some(DMTF_WSMAN_SCHEMA_NAMESPACE_URI));
define_tagname!(Release, Some(DMTF_WSMAN_SCHEMA_NAMESPACE_URI));
define_tagname!(GetStatus, Some(DMTF_WSMAN_SCHEMA_NAMESPACE_URI));

// WS-Management DMTF Headers (w namespace)
define_tagname!(ResourceURI, Some(DMTF_WSMAN_SCHEMA_NAMESPACE_URI));
define_tagname!(OperationTimeout, Some(DMTF_WSMAN_SCHEMA_NAMESPACE_URI));
define_tagname!(MaxEnvelopeSize, Some(DMTF_WSMAN_SCHEMA_NAMESPACE_URI));
define_tagname!(FragmentTransfer, Some(DMTF_WSMAN_SCHEMA_NAMESPACE_URI));
define_tagname!(SelectorSet, Some(DMTF_WSMAN_SCHEMA_NAMESPACE_URI));
define_tagname!(OptionSet, Some(DMTF_WSMAN_SCHEMA_NAMESPACE_URI));
define_tagname!(Locale, Some(DMTF_WSMAN_SCHEMA_NAMESPACE_URI));
define_custom_tagname!(
    OptionTagName,
    "Option",
    Some(DMTF_WSMAN_SCHEMA_NAMESPACE_URI)
);
define_custom_tagname!(
    SelectorTagName,
    "Selector",
    Some(DMTF_WSMAN_SCHEMA_NAMESPACE_URI)
);

// ===================================
// WS-Transfer (x namespace)
// ===================================
define_tagname!(Create, Some(WS_TRANSFER_NAMESPACE_URI));

define_tagname!(ResourceCreated, Some(WS_TRANSFER_NAMESPACE_URI));

// ====================================
// Microsoft WS-Management (p namespace)
// ====================================
define_tagname!(SequenceId, Some(MS_WSMAN_SCHEMA_NAMESPACE_URI));
define_tagname!(OperationID, Some(MS_WSMAN_SCHEMA_NAMESPACE_URI));
define_tagname!(SessionId, Some(MS_WSMAN_SCHEMA_NAMESPACE_URI));
define_tagname!(DataLocale, Some(MS_WSMAN_SCHEMA_NAMESPACE_URI));

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
