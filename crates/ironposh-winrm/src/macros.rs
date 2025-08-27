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
            fn append_to_element(self, mut element: ironposh_xml::builder::Element<$lifetime>) -> ironposh_xml::builder::Element<$lifetime> {
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

            impl<$lifetime> ironposh_xml::parser::XmlVisitor<$lifetime> for [<$struct_name Visitor>]<$lifetime> {
                type Value = $struct_name<$lifetime>;

                fn visit_children(
                    &mut self,
                    children: impl Iterator<Item = ironposh_xml::parser::Node<$lifetime, $lifetime>>,
                ) -> Result<(), ironposh_xml::XmlError> {
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
                                if let Ok(tag) = <$req_field_type as ironposh_xml::parser::XmlDeserialize>::from_node(child) {
                                    if self.$req_field.is_some() {
                                        return Err(ironposh_xml::XmlError::InvalidXml(format!(
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
                                if let Ok(tag) = <$opt_field_type as ironposh_xml::parser::XmlDeserialize>::from_node(child) {
                                    if self.$opt_field.is_some() {
                                        return Err(ironposh_xml::XmlError::InvalidXml(format!(
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
                            return Err(ironposh_xml::XmlError::InvalidXml(format!(
                                "Unknown tag in {}: {} (namespace: {:?})",
                                stringify!($struct_name),
                                tag_name,
                                namespace
                            )));
                        }
                    }

                    Ok(())
                }

                fn visit_node(&mut self, node: ironposh_xml::parser::Node<$lifetime, $lifetime>) -> Result<(), ironposh_xml::XmlError> {
                    let children: Vec<_> = node.children().collect();
                    self.visit_children(children.into_iter())?;
                    Ok(())
                }

                fn finish(self) -> Result<Self::Value, ironposh_xml::XmlError> {
                    // Required fields must be present
                    $(
                        let $req_field = self.$req_field.ok_or_else(|| {
                            ironposh_xml::XmlError::InvalidXml(format!(
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

            impl<$lifetime> ironposh_xml::parser::XmlDeserialize<$lifetime> for $struct_name<$lifetime> {
                type Visitor = [<$struct_name Visitor>]<$lifetime>;

                fn visitor() -> Self::Visitor {
                    [<$struct_name Visitor>]::default()
                }

                fn from_node(node: ironposh_xml::parser::Node<$lifetime, $lifetime>) -> Result<Self, ironposh_xml::XmlError> {
                    ironposh_xml::parser::NodeDeserializer::new(node).deserialize(Self::visitor())
                }
            }
        }
    };
}

#[macro_export]
macro_rules! impl_tag_from {
    ($src:ty => $taggen:ty) => {
        impl<'a, N> From<$src> for $taggen
        where
            N: TagName,
        {
            fn from(value: $src) -> Self {
                Tag::new(value)
            }
        }
    };
}

#[macro_export]
macro_rules! xml_num_value {
    ($name:ident, $inner:ty) => {
        paste::paste! {
            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            pub struct $name(pub $inner);

            // ------------ TagValue ---------------
            impl<'a> TagValue<'a> for $name {
                fn append_to_element(self, e: Element<'a>) -> Element<'a> { e.set_text(self.0.to_string()) }
            }

            // ------------ Visitor -----------------
            pub struct [<$name Visitor>] { value: Option<$name> }

            impl<'a> XmlVisitor<'a> for [<$name Visitor>] {
                type Value = $name;

                fn visit_node(&mut self, _n: ironposh_xml::parser::Node<'a, 'a>) -> Result<(), ironposh_xml::XmlError> { Ok(()) }

                fn visit_children(
                    &mut self,
                    children: impl Iterator<Item = ironposh_xml::parser::Node<'a, 'a>>,
                ) -> Result<(), ironposh_xml::XmlError> {
                    let nodes: Vec<_> = children.collect();
                    if nodes.len() != 1 {
                        return Err(ironposh_xml::XmlError::InvalidXml(
                            format!("{} expects exactly one text child", stringify!($name))
                        ));
                    }
                    if let Some(t) = nodes[0].text() {
                        self.value = Some($name(t.trim().parse::<$inner>().map_err(|_| {
                            ironposh_xml::XmlError::InvalidXml(format!("invalid {} value: {}", stringify!($name), t))
                        })?));
                    }
                    Ok(())
                }

                fn finish(self) -> Result<Self::Value, ironposh_xml::XmlError> {
                    self.value.ok_or(ironposh_xml::XmlError::InvalidXml(
                        format!("no {} value found", stringify!($name))
                    ))
                }
            }

            // ------------ Deserialize -------------
            impl<'a> XmlDeserialize<'a> for $name {
                type Visitor = [<$name Visitor>];
                fn visitor() -> Self::Visitor { [<$name Visitor>] { value: None } }
                fn from_node(node: ironposh_xml::parser::Node<'a, 'a>) -> Result<Self, ironposh_xml::XmlError> {
                    ironposh_xml::parser::NodeDeserializer::new(node).deserialize(Self::visitor())
                }
            }

            // ------------ Conversions -------------
            impl From<$inner> for $name        { fn from(v: $inner) -> Self { Self(v) } }
            impl From<$name>  for $inner       { fn from(v: $name)  -> Self { v.0 } }
        }
    }
}
