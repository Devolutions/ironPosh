use ironposh_macros::{PsDeserialize, PsSerialize};

/// SESSION_CAPABILITY message (MS-PSRP 2.2.2.1).
///
/// ```xml
/// <Obj RefId="0">
///    <MS>
///      <Version N="protocolversion">2.2</Version>
///      <Version N="PSVersion">2.0</Version>
///      <Version N="SerializationVersion">1.1.0.1</Version>
///      <BA N="TimeZone">...base64...</BA>
///    </MS>
///  </Obj>
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PsSerialize, PsDeserialize)]
#[ps(message_type = SessionCapability)]
pub struct SessionCapability {
    #[ps(name = "protocolversion", with = "version_conv")]
    pub protocol_version: String,
    #[ps(name = "PSVersion", with = "version_conv")]
    pub ps_version: String,
    #[ps(name = "SerializationVersion", with = "version_conv")]
    pub serialization_version: String,
    /// Opaque serialized .NET TimeZone blob, carried as a `<BA>` byte array.
    #[ps(name = "TimeZone", with = "timezone_conv")]
    pub time_zone: Option<String>,
}

/// `#[ps(with = ..)]` converter: these fields are .NET `Version` values
/// (`<Version>`), not plain strings.
mod version_conv {
    use crate::PowerShellRemotingError;
    use crate::ps_value::{PsPrimitiveValue, PsValue};

    pub fn to_ps_value(value: &str) -> PsValue {
        PsValue::Primitive(PsPrimitiveValue::Version(value.to_string()))
    }

    pub fn from_ps_value(value: &PsValue) -> Result<String, PowerShellRemotingError> {
        match value {
            PsValue::Primitive(PsPrimitiveValue::Version(v)) => Ok(v.clone()),
            other => Err(PowerShellRemotingError::InvalidMessage(format!(
                "expected Version, got {other:?}"
            ))),
        }
    }
}

/// `#[ps(with = ..)]` converter: the TimeZone is an opaque serialized blob the
/// client never interprets, carried as a `<BA>` byte array.
mod timezone_conv {
    use crate::PowerShellRemotingError;
    use crate::ps_value::{PsPrimitiveValue, PsValue};

    pub fn to_ps_value(value: &str) -> PsValue {
        PsValue::Primitive(PsPrimitiveValue::Bytes(value.as_bytes().to_vec()))
    }

    pub fn from_ps_value(value: &PsValue) -> Result<String, PowerShellRemotingError> {
        match value {
            PsValue::Primitive(PsPrimitiveValue::Bytes(bytes)) => {
                Ok(String::from_utf8_lossy(bytes).to_string())
            }
            other => Err(PowerShellRemotingError::InvalidMessage(format!(
                "expected ByteArray TimeZone, got {other:?}"
            ))),
        }
    }
}
