use crate::cores::Text;
use crate::tag;
use ironposh_macros::{FromXml, SimpleTagValue};

// Self-contained tags for exercising the SimpleTagValue + FromXml derives.
tag!(ReqA = Text<'a> => WsAddressing2004);
tag!(ReqB = Text<'a> => WsAddressing2004);
tag!(OptA = Text<'a> => WsAddressing2004);
tag!(OptB = Text<'a> => WsAddressing2004);

// Example struct with mixed required and optional fields using the derives.
#[derive(Debug, Clone, SimpleTagValue, FromXml)]
pub struct TestStruct<'a> {
    pub req_a: ReqA<'a>,         // Required
    pub req_b: ReqB<'a>,         // Required
    pub opt_a: Option<OptA<'a>>, // Optional
    pub opt_b: Option<OptB<'a>>, // Optional
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cores::{Tag, TagValue};
    use ironposh_xml::mapping::FromXml;

    const A: &str = "http://schemas.xmlsoap.org/ws/2004/08/addressing";

    #[test]
    fn test_serialization_and_deserialization_roundtrip() {
        let original = TestStruct {
            req_a: Tag::new(Text::from("a-value")),
            req_b: Tag::new(Text::from("b-value")),
            opt_a: Some(Tag::new(Text::from("opt-a-value"))),
            opt_b: None,
        };

        // Serialize via the TagValue derive (must not panic).
        let element = ironposh_xml::builder::Element::new("test");
        let _serialized = original.append_to_element(element);

        // Deserialize a namespaced document — matching is by URI.
        let xml = format!(
            r#"<test xmlns:a="{A}"><a:ReqA>a-value</a:ReqA><a:ReqB>b-value</a:ReqB><a:OptA>opt-a-value</a:OptA></test>"#
        );
        let doc = ironposh_xml::parser::parse(&xml).expect("parse");
        let parsed = TestStruct::from_xml(doc.root_element()).expect("deserialize");

        assert_eq!(parsed.req_a.value.as_ref(), "a-value");
        assert_eq!(parsed.req_b.value.as_ref(), "b-value");
        assert_eq!(parsed.opt_a.unwrap().value.as_ref(), "opt-a-value");
        assert!(parsed.opt_b.is_none());
    }

    #[test]
    fn test_deserialize_missing_required_field() {
        let xml = format!(r#"<test xmlns:a="{A}"><a:ReqA>only-a</a:ReqA></test>"#);
        let doc = ironposh_xml::parser::parse(&xml).expect("parse");
        assert!(TestStruct::from_xml(doc.root_element()).is_err());
    }
}
