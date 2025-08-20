use super::{
    ComplexObject, ComplexObjectContent, PsObjectWithType, PsPrimitiveValue, PsProperty, PsValue,
};
use crate::MessageType;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionCapability {
    pub protocol_version: String,
    pub ps_version: String,
    pub serialization_version: String,
    pub time_zone: Option<String>,
}

impl PsObjectWithType for SessionCapability {
    fn message_type(&self) -> MessageType {
        MessageType::SessionCapability
    }

    fn to_ps_object(&self) -> PsValue {
        PsValue::Object(ComplexObject::from(self.clone()))
    }
}

// <Obj RefId="0">
//    <MS>
//      <Version N="protocolversion">2.2</Version>
//      <Version N="PSVersion">2.0</Version>
//      <Version N="SerializationVersion">1.1.0.1</Version>
//      <BA N="TimeZone">AAEAAAD/////AQAAAAAAAAAEAQAAABxTeXN0ZW0uQ3VycmVudFN5c3RlbVRpbWVab25lBAAAABdtX0NhY2hlZERheWxpZ2h0Q2hhbmdlcw1tX3RpY2tzT2Zmc2V0Dm1fc3RhbmRhcmROYW1lDm1fZGF5bGlnaHROYW1lAwABARxTeXN0ZW0uQ29sbGVjdGlvbnMuSGFzaHRhYmxlCQkCAAAAAMDc8bz///8KCgQCAAAAHFN5c3RlbS5Db2xsZWN0aW9ucy5IYXNodGFibGUHAAAACkxvYWRGYWN0b3IHVmVyc2lvbghDb21wYXJlchBIYXNoQ29kZVByb3ZpZGVyCEhhc2hTaXplBEtleXMGVmFsdWVzAAADAwAFBQsIHFN5c3RlbS5Db2xsZWN0aW9ucy5JQ29tcGFyZXIkU3lzdGVtLkNvbGxlY3Rpb25zLklIYXNoQ29kZVByb3ZpZGVyCOxROD8BAAAACgoLAAAACQMAAAAJBAAAABADAAAAAQAAAAgI2QcAABAEAAAAAQAAAAkFAAAABAUAAAAhU3lzdGVtLkdsb2JhbGl6YXRpb24uRGF5bGlnaHRUaW1lAwAAAAdtX3N0YXJ0BW1fZW5kB21fZGVsdGEAAAANDQwAkOq4qG3LiAAQOyeuKMyIAGjEYQgAAAAL</BA>
//    </MS>
//  </Obj>

impl From<SessionCapability> for ComplexObject {
    fn from(cap: SessionCapability) -> Self {
        let mut extended_properties = BTreeMap::new();

        extended_properties.insert(
            "protocolversion".to_string(),
            PsProperty {
                name: "protocolversion".to_string(),
                value: PsValue::Primitive(PsPrimitiveValue::Version(cap.protocol_version)),
            },
        );

        extended_properties.insert(
            "PSVersion".to_string(),
            PsProperty {
                name: "PSVersion".to_string(),
                value: PsValue::Primitive(PsPrimitiveValue::Version(cap.ps_version)),
            },
        );

        extended_properties.insert(
            "SerializationVersion".to_string(),
            PsProperty {
                name: "SerializationVersion".to_string(),
                value: PsValue::Primitive(PsPrimitiveValue::Version(cap.serialization_version)),
            },
        );

        if let Some(time_zone) = cap.time_zone {
            extended_properties.insert(
                "TimeZone".to_string(),
                PsProperty {
                    name: "TimeZone".to_string(),
                    value: PsValue::Primitive(PsPrimitiveValue::Bytes(time_zone.into_bytes())),
                },
            );
        }

        ComplexObject {
            type_def: None,
            to_string: None,
            content: ComplexObjectContent::Standard,
            adapted_properties: BTreeMap::new(),
            extended_properties,
        }
    }
}

impl TryFrom<ComplexObject> for SessionCapability {
    type Error = crate::PowerShellRemotingError;

    fn try_from(value: ComplexObject) -> Result<Self, Self::Error> {
        let get_version_property = |name: &str| -> Result<String, Self::Error> {
            let property = value
                .extended_properties
                .get(name)
                .ok_or_else(|| Self::Error::InvalidMessage(format!("Missing property: {name}")))?;

            match &property.value {
                PsValue::Primitive(PsPrimitiveValue::Version(version)) => Ok(version.clone()),
                _ => Err(Self::Error::InvalidMessage(format!(
                    "Property '{name}' is not a Version"
                ))),
            }
        };

        let protocol_version = get_version_property("protocolversion")?;
        let ps_version = get_version_property("PSVersion")?;
        let serialization_version = get_version_property("SerializationVersion")?;

        let time_zone =
            value
                .extended_properties
                .get("TimeZone")
                .and_then(|prop| match &prop.value {
                    PsValue::Primitive(PsPrimitiveValue::Bytes(bytes)) => {
                        Some(String::from_utf8_lossy(bytes).to_string())
                    }
                    _ => None,
                });

        Ok(SessionCapability {
            protocol_version,
            ps_version,
            serialization_version,
            time_zone,
        })
    }
}
