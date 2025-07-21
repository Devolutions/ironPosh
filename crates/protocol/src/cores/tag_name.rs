use crate::cores::namespace::*;
use crate::{define_custom_tagname, define_tag_wrapper, define_tagname};

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

// PowerShell Remoting Operations
define_tagname!(Shell, Some(PWSH_NAMESPACE));
define_tagname!(Command, Some(PWSH_NAMESPACE));
define_tagname!(Receive, Some(PWSH_NAMESPACE));
define_tagname!(Send, Some(PWSH_NAMESPACE));
define_tagname!(Signal, Some(PWSH_NAMESPACE));

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
define_tagname!(Identify, Some(MS_WSMAN_NAMESPACE));
define_tagname!(Get, Some(MS_WSMAN_NAMESPACE));
define_tagname!(Put, Some(MS_WSMAN_NAMESPACE));
define_tagname!(Create, Some(MS_WSMAN_NAMESPACE));
define_tagname!(Delete, Some(MS_WSMAN_NAMESPACE));
define_tagname!(Enumerate, Some(MS_WSMAN_NAMESPACE));
define_tagname!(Pull, Some(MS_WSMAN_NAMESPACE));
define_tagname!(Release, Some(MS_WSMAN_NAMESPACE));
define_tagname!(GetStatus, Some(MS_WSMAN_NAMESPACE));

// WSMAN Headers
define_tagname!(ResourceURI, Some(MS_WSMAN_NAMESPACE));
define_tagname!(OperationTimeout, Some(MS_WSMAN_NAMESPACE));
define_tagname!(MaxEnvelopeSize, Some(MS_WSMAN_NAMESPACE));
define_tagname!(SequenceId, Some(MS_WSMAN_NAMESPACE));
define_tagname!(OperationID, Some(MS_WSMAN_NAMESPACE));
define_tagname!(FragmentTransfer, Some(MS_WSMAN_NAMESPACE));
define_tagname!(SelectorSet, Some(MS_WSMAN_NAMESPACE));
define_tagname!(SessionId, Some(MS_WSMAN_NAMESPACE));
define_tagname!(CompressionType, Some(MS_WSMAN_NAMESPACE));
define_tagname!(OptionSet, Some(MS_WSMAN_NAMESPACE));
define_custom_tagname!(OptionTagName, "Option", Some(MS_WSMAN_NAMESPACE));

// PowerShell Remoting Protocol;
define_tagname!(Obj, None);
define_tag_wrapper!(ObjWrapper, Obj);
define_tagname!(MS, None);
define_tag_wrapper!(MSWrapper, MS);
define_tagname!(Version, None);
define_tag_wrapper!(VersionWrapper, Version);
define_tagname!(BA, None);
define_tag_wrapper!(BAWrapper, BA);

// PowerShell Serialization Format
define_tagname!(I32, None);  // 32-bit integer
define_tag_wrapper!(I32Wrapper, I32);
define_tagname!(TN, None);   // Type Name
define_tag_wrapper!(TNWrapper, TN);
define_tagname!(T, None);    // Type
define_tag_wrapper!(TWrapper, T);
define_custom_tagname!(ToString, "ToString", None); // ToString representation
define_tag_wrapper!(ToStringWrapper, ToString);
define_tagname!(DCT, None);  // Dictionary
define_tag_wrapper!(DCTWrapper, DCT);
define_tagname!(En, None);   // Dictionary Entry
define_tag_wrapper!(EnWrapper, En);
define_tagname!(Key, None);  // Dictionary Key
define_tag_wrapper!(KeyWrapper, Key);
define_tagname!(Value, None); // Dictionary Value
define_tag_wrapper!(ValueWrapper, Value);
define_tagname!(Nil, None);  // Null Value
define_tag_wrapper!(NilWrapper, Nil);
define_tagname!(B, None);    // Boolean
define_tag_wrapper!(BWrapper, B);
define_tagname!(S, None);    // String
define_tag_wrapper!(SWrapper, S);
