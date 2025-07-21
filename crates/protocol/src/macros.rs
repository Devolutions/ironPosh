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
macro_rules! define_tag_wrapper {
    ($struct_name:ident, $tag_name_type:ty) => {
        pub struct $struct_name<'a, V>
        where
            V: $crate::cores::TagValue<'a>,
        {
            pub tag: $crate::cores::Tag<'a, V, $tag_name_type>,
        }

        impl<'a, V> $crate::cores::TagValue<'a> for $struct_name<'a, V>
        where
            V: $crate::cores::TagValue<'a>,
        {
            fn append_to_element(self, element: xml::builder::Element<'a>) -> xml::builder::Element<'a> {
                element.add_child(self.tag.into_element())
            }
        }

        impl<'a, V> $struct_name<'a, V>
        where
            V: $crate::cores::TagValue<'a>,
        {
            pub fn new(tag: $crate::cores::Tag<'a, V, $tag_name_type>) -> Self {
                Self { tag }
            }
        }

        impl<'a, V> xml::parser::XmlDeserialize<'a> for $struct_name<'a, V>
        where
            V: $crate::cores::TagValue<'a> + xml::parser::XmlDeserialize<'a>,
        {
            type Visitor = paste::paste! { [<$struct_name Visitor>]<'a, V> };

            fn visitor() -> Self::Visitor {
                paste::paste! { [<$struct_name Visitor>]::new() }
            }
        }

        paste::paste! {
            pub struct [<$struct_name Visitor>]<'a, V>
            where
                V: $crate::cores::TagValue<'a>,
            {
                tag: Option<$crate::cores::Tag<'a, V, $tag_name_type>>,
            }

            impl<'a, V> [<$struct_name Visitor>]<'a, V>
            where
                V: $crate::cores::TagValue<'a>,
            {
                pub fn new() -> Self {
                    Self { tag: None }
                }
            }

            impl<'a, V> xml::parser::XmlVisitor<'a> for [<$struct_name Visitor>]<'a, V>
            where
                V: $crate::cores::TagValue<'a> + xml::parser::XmlDeserialize<'a>,
            {
                type Value = $struct_name<'a, V>;

                fn visit_children(
                    &mut self,
                    node: impl Iterator<Item = xml::parser::Node<'a, 'a>>,
                ) -> Result<(), xml::XmlError<'a>> {
                    for child in node {
                        if !child.is_element() {
                            continue;
                        }

                        match (child.tag_name().name(), child.tag_name().namespace()) {
                            (<$tag_name_type>::TAG_NAME, <$tag_name_type>::NAMESPACE) => {
                                let tag = <$crate::cores::Tag<V, $tag_name_type> as xml::parser::XmlDeserialize>::from_node(child)?;
                                self.tag = Some(tag);
                            }
                            _ => {
                                tracing::warn!(
                                    "Unexpected child element in {}: {} (namespace: {:?})",
                                    stringify!($struct_name),
                                    child.tag_name().name(),
                                    child.tag_name().namespace()
                                );
                            }
                        }
                    }

                    if self.tag.is_none() {
                        return Err(xml::XmlError::InvalidXml(
                            format!("{} must contain a valid tag", stringify!($struct_name)),
                        ));
                    }

                    Ok(())
                }

                fn visit_node(&mut self, _node: xml::parser::Node<'a, 'a>) -> Result<(), xml::XmlError<'a>> {
                    Err(xml::XmlError::InvalidXml(
                        format!("{}Visitor should not be called with a single node", stringify!($struct_name)),
                    ))
                }

                fn finish(self) -> Result<Self::Value, xml::XmlError<'a>> {
                    self.tag
                        .map(|tag| $struct_name { tag })
                        .ok_or(xml::XmlError::InvalidXml(
                            format!("No valid tag found in {}Visitor", stringify!($struct_name)),
                        ))
                }
            }
        }
    };
}

