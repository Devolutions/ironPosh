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

            // ------------ Deserialize -------------
            impl<'a> ironposh_xml::mapping::FromXml<'a> for $name {
                fn from_xml(node: ironposh_xml::parser::Node<'a, 'a>) -> Result<Self, ironposh_xml::XmlError> {
                    let text = node.text().unwrap_or("").trim();
                    Ok($name(text.parse::<$inner>().map_err(|_| {
                        ironposh_xml::XmlError::InvalidXml(format!("invalid {} value: {}", stringify!($name), text))
                    })?))
                }
            }

            // ------------ Conversions -------------
            impl From<$inner> for $name        { fn from(v: $inner) -> Self { Self(v) } }
            impl From<$name>  for $inner       { fn from(v: $name)  -> Self { v.0 } }
        }
    }
}
