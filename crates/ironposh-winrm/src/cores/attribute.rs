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
            fn from_name_and_value(name: &str, value: &'a str) -> Result<Option<Self>, ironposh_xml::XmlError> {
                match name {
                    $(
                        $attr_name => {
                            match $parser(value) {
                                Ok(val) => Ok(Some(Attribute::$variant(val))),
                                Err(e) => Err(ironposh_xml::XmlError::InvalidXml(
                                    format!("Invalid value for {}: {}", $attr_name, e)
                                )),
                            }
                        }
                    )*
                    _ => Ok(None), // Unknown attribute, ignore
                }
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

// Define all attributes here - adding a new one automatically updates ALL related code
define_attributes!(
    MustUnderstand(bool) => (Some(crate::cores::namespace::Namespace::SoapEnvelope2003), "mustUnderstand"),
        |v: &str| v.parse::<bool>().map_err(|e| e.to_string()),
        |v: bool| v.to_string(),
    Name(Cow<'a, str>) => (None, "Name"),
        |v: &str| -> Result<Cow<'a, str>, String> { Ok(Cow::Owned(v.to_string())) },
        |v: Cow<'a, str>| v.into_owned(),
    MustComply(bool) => (None, "MustComply"),
        |v: &str| v.parse::<bool>().map_err(|e| e.to_string()),
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
        |v: &str| v.parse::<bool>().map_err(|e| e.to_string()),
        |v: bool| v.to_string(),
    Unit(Cow<'a, str>) => (None, "Unit"),
        |v: &str| -> Result<Cow<'a, str>, String> { Ok(Cow::Owned(v.to_string())) },
        |v: Cow<'a, str>| v.into_owned(),
    EndUnit(bool) => (None, "EndUnit"),
        |v: &str| v.parse::<bool>().map_err(|e| e.to_string()),
        |v: bool| v.to_string(),
    SequenceID(u64) => (None, "SequenceID"),
        |v: &str| v.parse::<u64>().map_err(|e| e.to_string()),
        |v: u64| v.to_string(),
    // Add new attributes here and they automatically get handled everywhere!
);

pub struct AttributeVisitor<'a> {
    attribute: Option<Attribute<'a>>,
}

impl<'a> ironposh_xml::parser::XmlVisitor<'a> for AttributeVisitor<'a> {
    type Value = Attribute<'a>;

    fn visit_attribute(
        &mut self,
        _attribute: ironposh_xml::parser::Attribute<'a, 'a>,
    ) -> Result<(), ironposh_xml::XmlError> {
        Attribute::from_name_and_value(_attribute.name(), _attribute.value())
            .map(|attr| {
                if let Some(parsed_attr) = attr {
                    self.attribute = Some(parsed_attr);
                }
            })
            .map_err(|e| ironposh_xml::XmlError::InvalidXml(e.to_string()))
    }

    fn finish(self) -> Result<Self::Value, ironposh_xml::XmlError> {
        self.attribute
            .ok_or_else(|| ironposh_xml::XmlError::InvalidXml("No attribute found".to_string()))
    }
}

impl<'a> ironposh_xml::parser::XmlDeserialize<'a> for Attribute<'a> {
    type Visitor = AttributeVisitor<'a>;

    fn visitor() -> Self::Visitor {
        AttributeVisitor { attribute: None }
    }

    fn from_node(
        _node: ironposh_xml::parser::Node<'a, 'a>,
    ) -> Result<Self, ironposh_xml::XmlError> {
        Err(ironposh_xml::XmlError::InvalidXml(
            "Attributes should not be parsed from nodes directly".to_string(),
        ))
    }

    fn from_children(
        _children: impl Iterator<Item = ironposh_xml::parser::Node<'a, 'a>>,
    ) -> Result<Self, ironposh_xml::XmlError> {
        Err(ironposh_xml::XmlError::InvalidXml(
            "Attributes should not be parsed from children directly".to_string(),
        ))
    }
}

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
            ("mustUnderstand", "true"),
            ("Name", "test-name"),
            ("MustComply", "false"),
        ];

        for (attr_name, attr_value) in test_cases {
            if let Some(parsed) = Attribute::from_name_and_value(attr_name, attr_value).unwrap() {
                // Test that we can get the name back
                assert_eq!(parsed.attribute_name(), attr_name);

                // Test round trip to XML attribute
                let _xml_attr: ironposh_xml::builder::Attribute = parsed.into();
            }
        }
    }
}
