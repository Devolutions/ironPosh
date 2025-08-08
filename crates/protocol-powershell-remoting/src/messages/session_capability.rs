use crate::{MessageType, PsObject, PsObjectWithType, PsProperty, PsValue};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionCapability {
    pub ref_id: u32,
    pub protocol_version: String,
    pub ps_version: String,
    pub serialization_version: String,
    pub time_zone: Option<String>,
}

impl PsObjectWithType for SessionCapability {
    fn message_type(&self) -> MessageType {
        MessageType::SessionCapability
    }

    fn to_ps_object(&self) -> PsObject {
        PsObject::from(self.clone())
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

impl From<SessionCapability> for PsObject {
    fn from(cap: SessionCapability) -> Self {
        let mut ms = vec![
            PsProperty {
                name: Some("protocolversion".to_string()),
                ref_id: None,
                value: PsValue::Version(cap.protocol_version),
            },
            PsProperty {
                name: Some("PSVersion".to_string()),
                ref_id: None,
                value: PsValue::Version(cap.ps_version),
            },
            PsProperty {
                name: Some("SerializationVersion".to_string()),
                ref_id: None,
                value: PsValue::Version(cap.serialization_version),
            },
        ];

        if let Some(time_zone) = cap.time_zone {
            ms.push(PsProperty {
                name: Some("TimeZone".to_string()),
                ref_id: None,
                value: PsValue::Bytes(time_zone.into_bytes()),
            });
        }

        PsObject {
            ref_id: cap.ref_id,
            ms,
            ..Default::default()
        }
    }
}
