use ironposh_macros::{PsDeserialize, PsSerialize};

/// Server → Client RUNSPACEPOOL_INIT_DATA message (MS-PSRP 2.2.2.13).
///
/// Returned inside the WSMan ConnectResponse `connectResponseXml` payload when
/// a new client attaches to a disconnected runspace pool shell. Carries the
/// pool's runspace limits.
///
/// ```xml
/// <Obj RefId="0">
///   <MS>
///     <I32 N="MinRunspaces">1</I32>
///     <I32 N="MaxRunspaces">1</I32>
///   </MS>
/// </Obj>
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PsSerialize, PsDeserialize)]
#[ps(message_type = RunspacepoolInitData)]
pub struct RunspacePoolInitData {
    #[ps(name = "MinRunspaces")]
    pub min_runspaces: i32,
    #[ps(name = "MaxRunspaces")]
    pub max_runspaces: i32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ps_value::{DeserializationContext, PsObjectWithType, PsValue, PsXmlDeserialize};

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
