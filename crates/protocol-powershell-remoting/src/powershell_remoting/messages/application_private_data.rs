use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "PascalCase",rename = "Obj")]
struct RootObj {
    #[serde(rename = "@RefId", skip_serializing_if = "Option::is_none")]
    pub ref_id: Option<u32>,
    #[serde(rename = "MS")]
    pub members: MemberSet,
}

#[derive(Debug, Deserialize, Serialize)]
struct MemberSet {
    #[serde(rename = "Obj")]
    pub objects: Vec<NamedObj>,
}

#[derive(Debug, Deserialize, Serialize)]
struct NamedObj {
    #[serde(rename = "@N")]
    pub name: String,
    #[serde(rename = "@RefId", skip_serializing_if = "Option::is_none")]
    pub ref_id: Option<u32>,
    #[serde(rename = "TN", skip_serializing_if = "Option::is_none")]
    pub type_names: Option<TypeNames>,
    #[serde(rename = "TNRef", skip_serializing_if = "Option::is_none")]
    pub tn_ref: Option<TypeNameRef>,
    #[serde(rename = "DCT", skip_serializing_if = "Option::is_none")]
    pub dictionary: Option<Dictionary>,
}

#[derive(Debug, Deserialize, Serialize)]
struct TypeNames {
    #[serde(rename = "@RefId", skip_serializing_if = "Option::is_none")]
    pub ref_id: Option<u32>,
    #[serde(rename = "T")]
    pub types: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
struct TypeNameRef {
    #[serde(rename = "@RefId")]
    pub ref_id: u32,
}

#[derive(Debug, Deserialize, Serialize)]
struct Dictionary {
    #[serde(rename = "En")]
    pub entries: Vec<Entry>,
}

#[derive(Debug, Deserialize, Serialize)]
struct Entry {
    #[serde(rename = "S", default, skip_serializing_if = "Option::is_none")]
    pub string_key: Option<NamedString>,

    #[serde(rename = "Version", default, skip_serializing_if = "Option::is_none")]
    pub version_value: Option<NamedVersion>,

    #[serde(rename = "Obj", default, skip_serializing_if = "Option::is_none")]
    pub nested_obj: Option<NamedObj>,
}

#[derive(Debug, Deserialize, Serialize)]
struct NamedString {
    #[serde(rename = "@N")]
    pub name: String,
    #[serde(rename = "$value")]
    pub value: String,
}

#[derive(Debug, Deserialize, Serialize)]
struct NamedVersion {
    #[serde(rename = "@N")]
    pub name: String,
    #[serde(rename = "$value")]
    pub value: String,
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
    <Obj N="ApplicationPrivateData" RefId="1">
      <TN RefId="0">
        <T>System.Management.Automation.PSPrimitiveDictionary</T>
        <T>System.Collections.Hashtable</T>
        <T>System.Object</T>
      </TN>
      <DCT>
        <En>
          <S N="Key">BashPrivateData</S>
          <Obj N="Value" RefId="2">
            <TNRef RefId="0" />
            <DCT>
              <En>
                <S N="Key">BashVersion</S>
                <Version N="Value">2.0</Version>
              </En>
            </DCT>
          </Obj>
        </En>
      </DCT>
    </Obj>
  </MS>
</Obj>
"#;

        let root: RootObj = from_str(xml).expect("Deserialization failed");

        // Basic checks to validate parsing
        assert_eq!(root.members.objects.len(), 1);

        let obj = &root.members.objects[0];
        assert_eq!(obj.name, "ApplicationPrivateData");

        let dict = obj.dictionary.as_ref().expect("Missing dictionary");
        assert_eq!(dict.entries.len(), 1);

        let entry = &dict.entries[0];
        let key = &entry.string_key.as_ref().unwrap().value;
        assert_eq!(key, "BashPrivateData");

        let nested_obj = entry.nested_obj.as_ref().unwrap();
        let nested_dict = nested_obj.dictionary.as_ref().unwrap();
        let inner_entry = &nested_dict.entries[0];
        let inner_key = &inner_entry.string_key.as_ref().unwrap().value;
        let version = &inner_entry.version_value.as_ref().unwrap().value;

        assert_eq!(inner_key, "BashVersion");
        assert_eq!(version, "2.0");
    }

    #[test]
    fn test_serialize() {
        let xml_obj = RootObj {
            ref_id: Some(0),
            members: MemberSet {
                objects: vec![NamedObj {
                    name: "ApplicationPrivateData".into(),
                    ref_id: Some(1),
                    type_names: Some(TypeNames {
                        ref_id: Some(0),
                        types: vec![
                            "System.Management.Automation.PSPrimitiveDictionary".into(),
                            "System.Collections.Hashtable".into(),
                            "System.Object".into(),
                        ],
                    }),
                    tn_ref: None,
                    dictionary: Some(Dictionary {
                        entries: vec![Entry {
                            string_key: Some(NamedString {
                                name: "Key".into(),
                                value: "BashPrivateData".into(),
                            }),
                            version_value: None,
                            nested_obj: Some(NamedObj {
                                name: "Value".into(),
                                ref_id: Some(2),
                                type_names: None,
                                tn_ref: Some(TypeNameRef { ref_id: 0 }),
                                dictionary: Some(Dictionary {
                                    entries: vec![Entry {
                                        string_key: Some(NamedString {
                                            name: "Key".into(),
                                            value: "BashVersion".into(),
                                        }),
                                        version_value: Some(NamedVersion {
                                            name: "Value".into(),
                                            value: "2.0".into(),
                                        }),
                                        nested_obj: None,
                                    }],
                                }),
                            }),
                        }],
                    }),
                }],
            },
        };

        let xml = to_string(&xml_obj).expect("Failed to serialize");
        println!("Serialized XML: {}", xml);

        // Test round-trip: deserialize the serialized XML
        let deserialized: RootObj = from_str(&xml).expect("Failed to deserialize serialized XML");
        
        // Verify the round-trip worked correctly
        assert_eq!(deserialized.members.objects.len(), 1);
        let obj = &deserialized.members.objects[0];
        assert_eq!(obj.name, "ApplicationPrivateData");
        assert_eq!(obj.ref_id, Some(1));
        
        let dict = obj.dictionary.as_ref().expect("Missing dictionary");
        assert_eq!(dict.entries.len(), 1);
        
        let entry = &dict.entries[0];
        let key = &entry.string_key.as_ref().unwrap().value;
        assert_eq!(key, "BashPrivateData");
        
        let nested_obj = entry.nested_obj.as_ref().unwrap();
        assert_eq!(nested_obj.name, "Value");
        assert_eq!(nested_obj.ref_id, Some(2));
        assert!(nested_obj.tn_ref.is_some());
        assert_eq!(nested_obj.tn_ref.as_ref().unwrap().ref_id, 0);
        
        let nested_dict = nested_obj.dictionary.as_ref().unwrap();
        let inner_entry = &nested_dict.entries[0];
        let inner_key = &inner_entry.string_key.as_ref().unwrap().value;
        let version = &inner_entry.version_value.as_ref().unwrap().value;
        
        assert_eq!(inner_key, "BashVersion");
        assert_eq!(version, "2.0");
    }
}
