use ironposh_macros::{PsDeserialize, PsSerialize};

/// Client → Server CONNECT_RUNSPACEPOOL message (MS-PSRP 2.2.2.14).
///
/// Sent inside the WSMan Connect `connectXml` payload when attaching a new
/// client to a disconnected runspace pool shell.
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
#[ps(message_type = ConnectRunspacepool)]
pub struct ConnectRunspacePool {
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
        let msg = ConnectRunspacePool {
            min_runspaces: 1,
            max_runspaces: 1,
        };
        assert_eq!(msg.message_type().value(), 0x0001_0008);
    }

    #[test]
    fn test_serialized_clixml_shape() {
        let msg = ConnectRunspacePool {
            min_runspaces: 1,
            max_runspaces: 4,
        };

        let xml = msg
            .to_ps_object()
            .to_element_as_root()
            .expect("serialize ConnectRunspacePool")
            .to_xml_string()
            .expect("xml string");

        assert!(xml.starts_with("<Obj"), "must serialize as <Obj>: {xml}");
        assert!(xml.contains("<MS>"), "must carry an <MS> section: {xml}");
        assert!(
            xml.contains(r#"<I32 N="MinRunspaces">1</I32>"#),
            "must carry MinRunspaces as I32: {xml}"
        );
        assert!(
            xml.contains(r#"<I32 N="MaxRunspaces">4</I32>"#),
            "must carry MaxRunspaces as I32: {xml}"
        );
    }

    #[test]
    fn test_roundtrip_parse() {
        let msg = ConnectRunspacePool {
            min_runspaces: 2,
            max_runspaces: 8,
        };

        let xml = msg
            .to_ps_object()
            .to_element_as_root()
            .expect("serialize ConnectRunspacePool")
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

        let roundtrip = ConnectRunspacePool::try_from(obj).expect("roundtrip parse");
        assert_eq!(msg, roundtrip);
    }
}
