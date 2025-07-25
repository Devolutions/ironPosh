#[macro_export]
macro_rules! define_custom_tagname {
    ($name:ident, $tagName:expr, $namespace:expr) => {
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct $name;

        impl $crate::cores::TagName for $name {
            const TAG_NAME: &'static str = $tagName;
            const NAMESPACE: Option<&'static str> = $namespace;

            fn tag_name(&self) -> &'static str {
                Self::TAG_NAME
            }

            fn namespace(&self) -> Option<&'static str> {
                Self::NAMESPACE
            }
        }

        impl<'a> $name {
            pub fn new_tag<V>(value: V) -> $crate::cores::tag::Tag<'a, V, Self>
            where
                V: $crate::cores::TagValue<'a>,
            {
                $crate::cores::tag::Tag::new(value)
            }
        }
    };
}

#[macro_export]
macro_rules! define_tagname {
    ($name:ident, $namespace:expr) => {
        $crate::define_custom_tagname!($name, stringify!($name), $namespace);
    };
}

#[macro_export]
macro_rules! impl_tag_value {
    (
        struct -> $struct_name:ident<$lifetime:lifetime>
        required -> [
            $(
                $req_field:ident
            ),* $(,)?
        ]
        optional -> [
            $(
                $opt_field:ident
            ),* $(,)?
        ]
    ) => {
        impl<$lifetime> $crate::cores::TagValue<$lifetime> for $struct_name<$lifetime> {
            fn append_to_element(self, mut element: xml::builder::Element<$lifetime>) -> xml::builder::Element<$lifetime> {
                // Append required fields
                $(
                    element = element.add_child(self.$req_field.into_element());
                )*
                
                // Append optional fields conditionally
                $(
                    element = match self.$opt_field {
                        Some(tag) => element.add_child(tag.into_element()),
                        None => element,
                    };
                )*
                
                element
            }
        }
    };
}

#[macro_export]
macro_rules! impl_xml_deserialize {
    (
        struct -> $struct_name:ident<$lifetime:lifetime>
        required -> [
            $(
                $req_field:ident: $req_field_type:ty
            ),* $(,)?
        ]
        optional -> [
            $(
                $opt_field:ident: $opt_field_type:ty
            ),* $(,)?
        ]
    ) => {
        paste::paste! {
            pub struct [<$struct_name Visitor>]<$lifetime> {
                $(
                    $req_field: Option<$req_field_type>,
                )*
                $(
                    $opt_field: Option<$opt_field_type>,  // This will be Option<Tag<...>> 
                )*
            }

            impl<$lifetime> Default for [<$struct_name Visitor>]<$lifetime> {
                fn default() -> Self {
                    Self {
                        $(
                            $req_field: None,
                        )*
                        $(
                            $opt_field: None,
                        )*
                    }
                }
            }

            impl<$lifetime> xml::parser::XmlVisitor<$lifetime> for [<$struct_name Visitor>]<$lifetime> {
                type Value = $struct_name<$lifetime>;

                fn visit_children(
                    &mut self,
                    children: impl Iterator<Item = xml::parser::Node<$lifetime, $lifetime>>,
                ) -> Result<(), xml::XmlError> {
                    for child in children {
                        if !child.is_element() {
                            continue;
                        }

                        let tag_name = child.tag_name().name();
                        let namespace = child.tag_name().namespace();

                        // We need a way to match tag names to field types
                        // For now, let's use a simpler approach where we try to deserialize each field type
                        let mut matched = false;

                        $(
                            // Try to match required fields
                            if !matched {
                                if let Ok(tag) = <$req_field_type as xml::parser::XmlDeserialize>::from_node(child) {
                                    if self.$req_field.is_some() {
                                        return Err(xml::XmlError::InvalidXml(format!(
                                            "Duplicate {} tag in {}", 
                                            tag_name,
                                            stringify!($struct_name)
                                        )));
                                    }
                                    self.$req_field = Some(tag);
                                    matched = true;
                                }
                            }
                        )*

                        $(
                            // Try to match optional fields - deserialize the Tag type and wrap in Some
                            if !matched {
                                if let Ok(tag) = <$opt_field_type as xml::parser::XmlDeserialize>::from_node(child) {
                                    if self.$opt_field.is_some() {
                                        return Err(xml::XmlError::InvalidXml(format!(
                                            "Duplicate {} tag in {}", 
                                            tag_name,
                                            stringify!($struct_name)
                                        )));
                                    }
                                    self.$opt_field = Some(tag);
                                    matched = true;
                                }
                            }
                        )*

                        if !matched {
                            return Err(xml::XmlError::InvalidXml(format!(
                                "Unknown tag in {}: {} (namespace: {:?})", 
                                stringify!($struct_name), 
                                tag_name,
                                namespace
                            )));
                        }
                    }

                    Ok(())
                }

                fn visit_node(&mut self, node: xml::parser::Node<$lifetime, $lifetime>) -> Result<(), xml::XmlError> {
                    let children: Vec<_> = node.children().collect();
                    self.visit_children(children.into_iter())?;
                    Ok(())
                }

                fn finish(self) -> Result<Self::Value, xml::XmlError> {
                    // Required fields must be present
                    $(
                        let $req_field = self.$req_field.ok_or_else(|| {
                            xml::XmlError::InvalidXml(format!(
                                "Missing {} in {}", 
                                stringify!($req_field), 
                                stringify!($struct_name)
                            ))
                        })?;
                    )*

                    // Optional fields - wrap in Some() if present, None if not
                    $(
                        let $opt_field = self.$opt_field;
                    )*

                    Ok($struct_name {
                        $(
                            $req_field,
                        )*
                        $(
                            $opt_field,
                        )*
                    })
                }
            }

            impl<$lifetime> xml::parser::XmlDeserialize<$lifetime> for $struct_name<$lifetime> {
                type Visitor = [<$struct_name Visitor>]<$lifetime>;

                fn visitor() -> Self::Visitor {
                    [<$struct_name Visitor>]::default()
                }

                fn from_node(node: xml::parser::Node<$lifetime, $lifetime>) -> Result<Self, xml::XmlError> {
                    xml::parser::NodeDeserializer::new(node).deserialize(Self::visitor())
                }
            }
        }
    };
}
