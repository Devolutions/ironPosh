use crate::MessageType;
use crate::ps_value::{
    ComplexObject, ComplexObjectContent, PsObjectWithType, PsPrimitiveValue, PsProperty, PsValue,
};
use std::collections::BTreeMap;

/// Server → Client RUNSPACEPOOL_INIT_DATA message (MS-PSRP 2.2.2.13).
///
/// Returned inside the WSMan ConnectResponse `connectResponseXml` payload when
/// a new client attaches to a disconnected runspace pool shell. Carries the
/// pool's runspace limits.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunspacePoolInitData {
    pub min_runspaces: i32,
    pub max_runspaces: i32,
}

impl PsObjectWithType for RunspacePoolInitData {
    fn message_type(&self) -> MessageType {
        MessageType::RunspacepoolInitData
    }

    fn to_ps_object(&self) -> PsValue {
        PsValue::Object(ComplexObject::from(self.clone()))
    }
}

// <Obj RefId="0">
//   <MS>
//     <I32 N="MinRunspaces">1</I32>
//     <I32 N="MaxRunspaces">1</I32>
//   </MS>
// </Obj>
impl From<RunspacePoolInitData> for ComplexObject {
    fn from(value: RunspacePoolInitData) -> Self {
        let mut extended_properties = BTreeMap::new();

        extended_properties.insert(
            "MinRunspaces".to_string(),
            PsProperty {
                name: "MinRunspaces".to_string(),
                value: PsValue::Primitive(PsPrimitiveValue::I32(value.min_runspaces)),
            },
        );

        extended_properties.insert(
            "MaxRunspaces".to_string(),
            PsProperty {
                name: "MaxRunspaces".to_string(),
                value: PsValue::Primitive(PsPrimitiveValue::I32(value.max_runspaces)),
            },
        );

        Self {
            type_def: None,
            to_string: None,
            content: ComplexObjectContent::Standard,
            adapted_properties: BTreeMap::new(),
            extended_properties,
        }
    }
}

impl TryFrom<ComplexObject> for RunspacePoolInitData {
    type Error = crate::PowerShellRemotingError;

    fn try_from(value: ComplexObject) -> Result<Self, Self::Error> {
        let get_i32_property = |name: &str| -> Result<i32, Self::Error> {
            let property = value
                .extended_properties
                .get(name)
                .ok_or_else(|| Self::Error::InvalidMessage(format!("Missing property: {name}")))?;

            match &property.value {
                PsValue::Primitive(PsPrimitiveValue::I32(v)) => Ok(*v),
                other => Err(Self::Error::InvalidMessage(format!(
                    "Property '{name}' must be an I32, got {other:?}"
                ))),
            }
        };

        Ok(Self {
            min_runspaces: get_i32_property("MinRunspaces")?,
            max_runspaces: get_i32_property("MaxRunspaces")?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ps_value::{DeserializationContext, PsXmlDeserialize};

    #[test]
    fn test_message_type() {
        let msg = RunspacePoolInitData {
            min_runspaces: 1,
            max_runspaces: 1,
        };
        assert_eq!(msg.message_type().value(), 0x0002_100B);
    }

    #[test]
    fn test_parse_from_clixml() {
        let xml = r#"<Obj RefId="0"><MS><I32 N="MinRunspaces">1</I32><I32 N="MaxRunspaces">3</I32></MS></Obj>"#;

        let parsed = ironposh_xml::parser::parse(xml).expect("parse xml");
        let ps_value = PsValue::from_node_with_context(
            parsed.root_element(),
            &mut DeserializationContext::default(),
        )
        .expect("deserialize PsValue");

        let PsValue::Object(obj) = ps_value else {
            panic!("expected PsValue::Object");
        };

        let init_data = RunspacePoolInitData::try_from(obj).expect("parse RunspacePoolInitData");
        assert_eq!(init_data.min_runspaces, 1);
        assert_eq!(init_data.max_runspaces, 3);
    }

    #[test]
    fn test_roundtrip_parse() {
        let msg = RunspacePoolInitData {
            min_runspaces: 1,
            max_runspaces: 5,
        };

        let xml = msg
            .to_ps_object()
            .to_element_as_root()
            .expect("serialize RunspacePoolInitData")
            .to_xml_string()
            .expect("xml string");

        let parsed = ironposh_xml::parser::parse(&xml).expect("parse xml");
        let ps_value = PsValue::from_node_with_context(
            parsed.root_element(),
            &mut DeserializationContext::default(),
        )
        .expect("deserialize PsValue");

        let PsValue::Object(obj) = ps_value else {
            panic!("expected PsValue::Object");
        };

        let roundtrip = RunspacePoolInitData::try_from(obj).expect("roundtrip parse");
        assert_eq!(msg, roundtrip);
    }
}
