use serde::{Deserialize, Serialize};

/// SESSION_CAPABILITY Message (MessageType: 0x00010002)
///
/// The Data field contains UTF-8 encoded XML representing a Complex Object
/// with extended properties for PowerShell session capabilities.
#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase")]
pub struct SessionCapability {
    #[serde(rename = "@RefId", skip_serializing_if = "Option::is_none")]
    pub ref_id: Option<u32>,
    #[serde(rename = "MS")]
    pub members: MemberSet,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct MemberSet {
    #[serde(rename = "Version")]
    pub versions: Vec<VersionValue>,

    /// Time zone of the client (optional)
    #[serde(rename = "BA", skip_serializing_if = "Option::is_none")]
    pub time_zone: Option<ByteArrayValue>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct VersionValue {
    #[serde(rename = "@N")]
    pub name: String,
    #[serde(rename = "$value")]
    pub value: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ByteArrayValue {
    #[serde(rename = "@N")]
    pub name: String,
    #[serde(rename = "$value")]
    pub value: String,
}

impl SessionCapability {
    /// Create a new SessionCapability with the specified versions
    pub fn new(
        protocol_version: &str,
        ps_version: &str,
        serialization_version: &str,
        time_zone: Option<String>,
    ) -> Self {
        let versions = vec![
            VersionValue {
                name: "protocolversion".to_string(),
                value: protocol_version.to_string(),
            },
            VersionValue {
                name: "PSVersion".to_string(),
                value: ps_version.to_string(),
            },
            VersionValue {
                name: "SerializationVersion".to_string(),
                value: serialization_version.to_string(),
            },
        ];

        let members = MemberSet {
            versions,
            time_zone: time_zone.map(|tz| ByteArrayValue {
                name: "TimeZone".to_string(),
                value: tz,
            }),
        };

        SessionCapability {
            ref_id: Some(0),
            members,
        }
    }

    /// Get the protocol version
    pub fn protocol_version(&self) -> Option<&str> {
        self.members
            .versions
            .iter()
            .find(|v| v.name == "protocolversion")
            .map(|v| v.value.as_str())
    }

    /// Get the PowerShell version
    pub fn ps_version(&self) -> Option<&str> {
        self.members
            .versions
            .iter()
            .find(|v| v.name == "PSVersion")
            .map(|v| v.value.as_str())
    }

    /// Get the serialization version
    pub fn serialization_version(&self) -> Option<&str> {
        self.members
            .versions
            .iter()
            .find(|v| v.name == "SerializationVersion")
            .map(|v| v.value.as_str())
    }

    /// Get the time zone (if present)
    pub fn time_zone(&self) -> Option<&str> {
        self.members.time_zone.as_ref().map(|tz| tz.value.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quick_xml::de::from_str;
    use quick_xml::se::to_string;

    #[test]
    fn test_deserialize() {
        let xml = r#"
<Obj RefId="0">
  <MS>
    <Version N="protocolversion">2.2</Version>
    <Version N="PSVersion">2.0</Version>
    <Version N="SerializationVersion">1.1.0.1</Version>
    <BA N="TimeZone">AAEAAAD/////AQAAAAAAAAAEAQAAABxTeXN0ZW0uQ3VycmVudFN5c3RlbVRpbWVab25lBAAAABdtX0NhY2hlZERheWxpZ2h0Q2hhbmdlcw1tX3RpY2tzT2Zmc2V0Dm1fc3RhbmRhcmROYW1lDm1fZGF5bGlnaHROYW1lAwABERxTeXN0ZW0uQ29sbGVjdGlvbnMuSGFzaHRhYmxlCQkCAAAAAMDc8bz///8KCgQCAAAAHFN5c3RlbS5Db2xsZWN0aW9ucy5IYXNodGFibGUHAAAACkxvYWRGYWN0b3IHVmVyc2lvbghDb21wYXJlchBIYXNoQ29kZVByb3ZpZGVyCEhhc2hTaXplBEtleXMGVmFsdWVzAAADAwAFBQsIHFN5c3RlbS5Db2xsZWN0aW9ucy5JQ29tcGFyZXIkU3lzdGVtLkNvbGxlY3Rpb25zLklIYXNoQ29kZVByb3ZpZGVyCOxROD8BAAAACgoLAAAACQMAAAAJBAAAABADAAAAAQAAAAgI2QcAABAEAAAAAQAAAAkFAAAABAUAAAAhU3lzdGVtLkdsb2JhbGl6YXRpb24uRGF5bGlnaHRUaW1lAwAAAAdtX3N0YXJ0BW1fZW5kB21fZGVsdGEAAAANDQwAkOq4qG3LiAAQOyeuKMyIAGjEYQgAAAAL</BA>
  </MS>
</Obj>
"#;

        let session_capability: SessionCapability = from_str(xml).expect("Deserialization failed");

        // Verify basic structure
        assert_eq!(session_capability.ref_id, Some(0));

        // Verify protocol version
        assert_eq!(session_capability.protocol_version(), Some("2.2"));

        // Verify PowerShell version
        assert_eq!(session_capability.ps_version(), Some("2.0"));

        // Verify serialization version
        assert_eq!(session_capability.serialization_version(), Some("1.1.0.1"));

        // Verify time zone is present
        assert!(session_capability.time_zone().is_some());

        // Verify we have 3 versions
        assert_eq!(session_capability.members.versions.len(), 3);
    }

    #[test]
    fn test_serialize() {
        let session_capability = SessionCapability::new(
            "2.2",
            "2.0", 
            "1.1.0.1",
            Some("AAEAAAD/////AQAAAAAAAAAEAQAAABxTeXN0ZW0uQ3VycmVudFN5c3RlbVRpbWVab25lBAAAABdtX0NhY2hlZERheWxpZ2h0Q2hhbmdlcw1tX3RpY2tzT2Zmc2V0Dm1fc3RhbmRhcmROYW1lDm1fZGF5bGlnaHROYW1lAwABERxTeXN0ZW0uQ29sbGVjdGlvbnMuSGFzaHRhYmxlCQkCAAAAAMDc8bz///8KCgQCAAAAHFN5c3RlbS5Db2xsZWN0aW9ucy5IYXNodGFibGUHAAAACkxvYWRGYWN0b3IHVmVyc2lvbghDb21wYXJlchBIYXNoQ29kZVByb3ZpZGVyCEhhc2hTaXplBEtleXMGVmFsdWVzAAADAwAFBQsIHFN5c3RlbS5Db2xsZWN0aW9ucy5JQ29tcGFyZXIkU3lzdGVtLkNvbGxlY3Rpb25zLklIYXNoQ29kZVByb3ZpZGVyCOxROD8BAAAACgoLAAAACQMAAAAJBAAAABADAAAAAQAAAAgI2QcAABAEAAAAAQAAAAkFAAAABAUAAAAhU3lzdGVtLkdsb2JhbGl6YXRpb24uRGF5bGlnaHRUaW1lAwAAAAdtX3N0YXJ0BW1fZW5kB21fZGVsdGEAAAANDQwAkOq4qG3LiAAQOyeuKMyIAGjEYQgAAAAL".to_string())
        );

        let xml = to_string(&session_capability).expect("Failed to serialize");
        println!("Serialized XML: {}", xml);

        // Test round-trip: deserialize the serialized XML
        let deserialized: SessionCapability =
            from_str(&xml).expect("Failed to deserialize serialized XML");

        // Verify the round-trip worked correctly
        assert_eq!(deserialized.protocol_version(), Some("2.2"));
        assert_eq!(deserialized.ps_version(), Some("2.0"));
        assert_eq!(deserialized.serialization_version(), Some("1.1.0.1"));
        assert!(deserialized.time_zone().is_some());
    }

    #[test]
    fn test_without_timezone() {
        let session_capability = SessionCapability::new("2.3", "5.1", "1.2.0.0", None);

        let xml = to_string(&session_capability).expect("Failed to serialize");
        let deserialized: SessionCapability = from_str(&xml).expect("Failed to deserialize");

        assert_eq!(deserialized.protocol_version(), Some("2.3"));
        assert_eq!(deserialized.ps_version(), Some("5.1"));
        assert_eq!(deserialized.serialization_version(), Some("1.2.0.0"));
        assert!(deserialized.time_zone().is_none());
    }
}
