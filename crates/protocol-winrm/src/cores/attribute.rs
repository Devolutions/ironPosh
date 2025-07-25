use std::borrow::Cow;

// Macro that ensures compile-time safety by generating all the boilerplate
macro_rules! define_attributes {
    (
        $(
            $variant:ident($type:ty) => ($namespace:expr, $attr_name:literal), $parser:expr
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
            fn from_name_and_value(name: &str, value: &'a str) -> Result<Option<Self>, xml::XmlError> {
                match name {
                    $(
                        $attr_name => {
                            match $parser(value) {
                                Ok(val) => Ok(Some(Attribute::$variant(val))),
                                Err(e) => Err(xml::XmlError::InvalidXml(
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

        impl<'a> From<Attribute<'a>> for xml::builder::Attribute<'a> {
            fn from(val: Attribute<'a>) -> Self {
                let namespace = val.namespace().map(|ns| {
                    let (uri, _alias) = ns.as_tuple();
                    xml::builder::Namespace::new(uri)
                });

                match val {
                    $(
                        Attribute::$variant(value) => {
                            let attr = xml::builder::Attribute::new($attr_name, Cow::Owned(format!("{}", value)));
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
    MustUnderstand(bool) => (Some(crate::cores::namespace::Namespace::Soap), "mustUnderstand"), |v: &str| v.parse::<bool>().map_err(|e| e.to_string()),
    Name(Cow<'a, str>) => (None, "Name"), |v: &str| -> Result<Cow<'a, str>, String> { Ok(Cow::Owned(v.to_string())) },
    MustComply(bool) => (Some(crate::cores::namespace::Namespace::WsManagement), "MustComply"), |v: &str| v.parse::<bool>().map_err(|e| e.to_string()),
    ShellId(Cow<'a, str>) => (Some(crate::cores::namespace::Namespace::RspShell), "ShellId"), |v: &str| -> Result<Cow<'a, str>, String> { Ok(Cow::Owned(v.to_string())) },
    RefId(Cow<'a, str>) => (None, "RefId"), |v: &str| -> Result<Cow<'a, str>, String> { Ok(Cow::Owned(v.to_string())) },
    N(Cow<'a, str>) => (None, "N"), |v: &str| -> Result<Cow<'a, str>, String> { Ok(Cow::Owned(v.to_string())) },
    // Add new attributes here and they automatically get handled everywhere!
);

pub struct AttributeVisitor<'a> {
    attribute: Option<Attribute<'a>>,
}

impl<'a> xml::parser::XmlVisitor<'a> for AttributeVisitor<'a> {
    type Value = Attribute<'a>;

    fn visit_children(
        &mut self,
        _node: impl Iterator<Item = xml::parser::Node<'a, 'a>>,
    ) -> Result<(), xml::XmlError> {
        Err(xml::XmlError::InvalidXml(
            "Expected no children for Attribute".to_string(),
        ))
    }

    fn visit_node(&mut self, node: xml::parser::Node<'a, 'a>) -> Result<(), xml::XmlError> {
        for attribute in node.attributes() {
            tracing::debug!(
                "AttributeVisitor checking attribute: name='{}', value='{}'",
                attribute.name(),
                attribute.value()
            );

            // Try to parse this attribute using our compile-time safe method
            if let Some(parsed_attr) =
                Attribute::from_name_and_value(attribute.name(), attribute.value())?
            {
                self.attribute = Some(parsed_attr);
                tracing::debug!("Successfully parsed attribute: {:?}", self.attribute);
                return Ok(()); // Take the first recognized attribute
            } else {
                tracing::debug!("Ignoring unknown attribute: {}", attribute.name());
            }
        }

        Err(xml::XmlError::InvalidXml(
            "No valid attribute found".to_string(),
        ))
    }

    fn finish(self) -> Result<Self::Value, xml::XmlError> {
        self.attribute
            .ok_or(xml::XmlError::InvalidXml("No attribute found".to_string()))
    }
}

impl<'a> xml::parser::XmlDeserialize<'a> for Attribute<'a> {
    type Visitor = AttributeVisitor<'a>;

    fn visitor() -> Self::Visitor {
        AttributeVisitor { attribute: None }
    }

    fn from_node(node: xml::parser::Node<'a, 'a>) -> Result<Self, xml::XmlError> {
        xml::parser::NodeDeserializer::new(node).deserialize(Self::visitor())
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
            let _xml_attr: xml::builder::Attribute = attr.clone().into();
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
                let _xml_attr: xml::builder::Attribute = parsed.into();
            }
        }
    }
}
