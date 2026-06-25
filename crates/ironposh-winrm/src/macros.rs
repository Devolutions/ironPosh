/// Defines a WinRM/SOAP tag.
///
/// Generates a zero-sized `TagName` marker (`<Alias>Tag`) that pins the element's
/// wire name + namespace at the type level, plus an ergonomic type alias
/// `Alias<'a> = Tag<'a, Value, AliasTag>` used in struct fields and construction.
/// The marker stays internal; everything else writes the alias.
///
/// Forms:
/// - `tag!(Get = Text<'a> => DmtfWsmanSchema)` — wire name is the alias ident.
/// - `tag!(SoapValue = "Value": Text<'a> => SoapEnvelope2003)` — custom wire name
///   (and lets several aliases share one wire name, e.g. `LocaleEmpty`/`LocaleText`).
#[macro_export]
macro_rules! tag {
    ($alias:ident = $value:ty => $ns:ident) => {
        $crate::tag!(@build $alias, stringify!($alias), $value, $ns);
    };
    ($alias:ident = $wire:literal : $value:ty => $ns:ident) => {
        $crate::tag!(@build $alias, $wire, $value, $ns);
    };
    (@build $alias:ident, $wire:expr, $value:ty, $ns:ident) => {
        paste::paste! {
            #[derive(Debug, Clone, PartialEq, Eq)]
            pub struct [<$alias Tag>];

            impl $crate::cores::TagName for [<$alias Tag>] {
                const TAG_NAME: &'static str = $wire;
                const NAMESPACE: Option<&'static str> =
                    ::core::option::Option::Some($crate::cores::Namespace::$ns.uri());
            }

            pub type $alias<'a> = $crate::cores::Tag<'a, $value, [<$alias Tag>]>;
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

            // ------------ Deserialize -------------
            impl<'a> ironposh_xml::mapping::FromXml<'a> for $name {
                fn from_xml(node: ironposh_xml::parser::Node<'a, 'a>) -> Result<Self, ironposh_xml::XmlError> {
                    let text = $crate::cores::tag_value::leaf_text(node)?;
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
