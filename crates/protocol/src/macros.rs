#[macro_export]
macro_rules! define_custom_tagname {
    ($name:ident, $tagName:expr, $namespace:expr) => {
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct $name;

        impl $crate::cores::TagName for $name {
            const TAG_NAME: &'static str = $tagName;
            const NAMESPACE: Option<&'static str> = $namespace;
        }

        impl<'a> $name {
            pub fn new_tag<V>(value: V) -> $crate::cores::tag::Tag<'a, V, Self>
            where
                V: $crate::cores::TagValue<'a>,
            {
                $crate::cores::tag::Tag::new(value.into())
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

