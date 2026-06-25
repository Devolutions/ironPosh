use std::borrow::Cow;

use uuid::Uuid;

// Macro that ensures compile-time safety by generating all the boilerplate
macro_rules! define_attributes {
    (
        $(
            $variant:ident($type:ty) => ($namespace:expr, $attr_name:literal), $parser:expr, $serializer:expr
        ),* $(,)?
    ) => {
        #[derive(Debug, Clone)]
        pub enum Attribute<'a> {
            $(
                $variant($type),
            )*
        }

        impl<'a> Attribute<'a> {
            /// Convert an attribute name to the corresponding enum variant type
            /// This is automatically generated to match all enum variants
            pub fn from_name_and_value(namespace: Option<&str>, name: &str, value: &'a str) -> Result<Option<Self>, ironposh_xml::XmlError> {
                // The reserved `xml:` prefix is modeled literally (e.g. "xml:lang",
                // no declared namespace); fold roxmltree's expanded
                // (xml-namespace, local) form back to that spelling so it matches.
                let xml_prefixed;
                let (namespace, name) = if namespace == Some("http://www.w3.org/XML/1998/namespace") {
                    xml_prefixed = format!("xml:{name}");
                    (None, xml_prefixed.as_str())
                } else {
                    (namespace, name)
                };
                // Identity is the (namespace-URI, local-name) pair, like elements:
                // a known attribute in the wrong namespace is not that attribute.
                $(
                    {
                        let expected_ns: Option<crate::cores::namespace::Namespace> = $namespace;
                        if name == $attr_name && namespace == expected_ns.map(|ns| ns.uri()) {
                            return match $parser(value) {
                                Ok(val) => Ok(Some(Attribute::$variant(val))),
                                Err(e) => Err(ironposh_xml::XmlError::InvalidXml(
                                    format!("Invalid value for {}: {}", $attr_name, e)
                                )),
                            };
                        }
                    }
                )*
                Ok(None) // Unknown attribute, ignore
            }

            /// Get the attribute name for this enum variant
            /// This is automatically generated to be exhaustive
            pub fn attribute_name(&self) -> &'static str {
                match self {
                    $(
                        Attribute::$variant(_) => $attr_name,
                    )*
                }
            }

            /// Get the namespace for this enum variant
            /// This is automatically generated to be exhaustive
            pub fn namespace(&self) -> Option<crate::cores::namespace::Namespace> {
                match self {
                    $(
                        Attribute::$variant(_) => $namespace,
                    )*
                }
            }
        }

        impl<'a> From<Attribute<'a>> for ironposh_xml::builder::Attribute<'a> {
            fn from(val: Attribute<'a>) -> Self {
                let namespace = val.namespace().map(|ns| {
                    let (uri, _alias) = ns.as_tuple();
                    ironposh_xml::builder::Namespace::new(uri)
                });

                match val {
                    $(
                        Attribute::$variant(value) => {
                            let serialized_value = $serializer(value);
                            let attr = ironposh_xml::builder::Attribute::new($attr_name, Cow::Owned(serialized_value));
                            if let Some(ns) = namespace {
                                attr.set_namespace(ns)
                            } else {
                                attr
                            }
                        }
                    )*
                }
            }
        }

    };
}

/// XSD boolean: `true`/`false` and their `1`/`0` aliases (MS-WSMV/SOAP send both).
fn parse_xml_bool(v: &str) -> Result<bool, String> {
    match v {
        "true" | "1" => Ok(true),
        "false" | "0" => Ok(false),
        other => Err(format!(
            "expected xs:boolean (true/false/1/0), got {other:?}"
        )),
    }
}

// Define all attributes here - adding a new one automatically updates ALL related code
define_attributes!(
    MustUnderstand(bool) => (Some(crate::cores::namespace::Namespace::SoapEnvelope2003), "mustUnderstand"),
        parse_xml_bool,
        |v: bool| v.to_string(),
    Name(Cow<'a, str>) => (None, "Name"),
        |v: &str| -> Result<Cow<'a, str>, String> { Ok(Cow::Owned(v.to_string())) },
        |v: Cow<'a, str>| v.into_owned(),
    MustComply(bool) => (None, "MustComply"),
        parse_xml_bool,
        |v: bool| v.to_string(),
    ShellId(Cow<'a, str>) => (None, "ShellId"),
        |v: &str| -> Result<Cow<'a, str>, String> { Ok(Cow::Owned(v.to_string())) },
        |v: Cow<'a, str>| v.into_owned(),
    RefId(Cow<'a, str>) => (None, "RefId"),
        |v: &str| -> Result<Cow<'a, str>, String> { Ok(Cow::Owned(v.to_string())) },
        |v: Cow<'a, str>| v.into_owned(),
    N(Cow<'a, str>) => (None, "N"),
        |v: &str| -> Result<Cow<'a, str>, String> { Ok(Cow::Owned(v.to_string())) },
        |v: Cow<'a, str>| v.into_owned(),
    XmlLang(Cow<'a, str>) => (None, "xml:lang"),
        |v: &str| -> Result<Cow<'a, str>, String> { Ok(Cow::Owned(v.to_string())) },
        |v: Cow<'a, str>| v.into_owned(),
    CommandId(Uuid) => (None, "CommandId"),
        |v: &str| -> Result<Uuid, String> {
            Uuid::parse_str(v).map_err(|e| e.to_string())
        },
        |v: Uuid| v.to_string().to_uppercase(),
    State(Cow<'a, str>) => (None, "State"),
        |v: &str| -> Result<Cow<'a, str>, String> { Ok(Cow::Owned(v.to_string())) },
        |v: Cow<'a, str>| v.into_owned(),
    End(bool) => (None, "End"),
        parse_xml_bool,
        |v: bool| v.to_string(),
    Unit(Cow<'a, str>) => (None, "Unit"),
        |v: &str| -> Result<Cow<'a, str>, String> { Ok(Cow::Owned(v.to_string())) },
        |v: Cow<'a, str>| v.into_owned(),
    EndUnit(bool) => (None, "EndUnit"),
        parse_xml_bool,
        |v: bool| v.to_string(),
    SequenceID(u64) => (None, "SequenceID"),
        |v: &str| v.parse::<u64>().map_err(|e| e.to_string()),
        |v: u64| v.to_string(),
    // Add new attributes here and they automatically get handled everywhere!
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compile_time_exhaustiveness() {
        // This test demonstrates that the macro generates exhaustive matches
        let attrs = vec![
            Attribute::MustUnderstand(true),
            Attribute::Name(Cow::Borrowed("test")),
            Attribute::MustComply(false),
            // If you add a new variant, the macro will automatically handle it
        ];

        for attr in attrs {
            let _name = attr.attribute_name();
            let _xml_attr: ironposh_xml::builder::Attribute = attr.clone().into();
        }
    }

    #[test]
    fn test_parsing_round_trip() {
        let test_cases = [
            (
                Some(crate::cores::namespace::Namespace::SoapEnvelope2003.uri()),
                "mustUnderstand",
                "true",
            ),
            (None, "Name", "test-name"),
            (None, "MustComply", "false"),
        ];

        for (ns, attr_name, attr_value) in test_cases {
            if let Some(parsed) = Attribute::from_name_and_value(ns, attr_name, attr_value).unwrap()
            {
                // Test that we can get the name back
                assert_eq!(parsed.attribute_name(), attr_name);

                // Test round trip to XML attribute
                let _xml_attr: ironposh_xml::builder::Attribute = parsed.into();
            }
        }
    }

    #[test]
    fn known_attribute_in_wrong_namespace_is_unmatched() {
        // `mustUnderstand` lives in the SOAP namespace; unqualified it is unknown.
        assert!(
            Attribute::from_name_and_value(None, "mustUnderstand", "true")
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn known_attribute_with_bad_value_propagates_error() {
        assert!(Attribute::from_name_and_value(None, "MustComply", "notabool").is_err());
    }

    #[test]
    fn xsd_boolean_accepts_one_and_zero() {
        assert!(matches!(
            Attribute::from_name_and_value(None, "MustComply", "1").unwrap(),
            Some(Attribute::MustComply(true))
        ));
        assert!(matches!(
            Attribute::from_name_and_value(
                Some(crate::cores::namespace::Namespace::SoapEnvelope2003.uri()),
                "mustUnderstand",
                "0"
            )
            .unwrap(),
            Some(Attribute::MustUnderstand(false))
        ));
    }

    #[test]
    fn xml_lang_matches_via_expanded_namespace() {
        let parsed = Attribute::from_name_and_value(
            Some("http://www.w3.org/XML/1998/namespace"),
            "lang",
            "en-US",
        )
        .unwrap();
        assert!(matches!(parsed, Some(Attribute::XmlLang(_))));
    }
}
