#[macro_export]
macro_rules! opt_header {
    ($vec:ident, $($field:expr),* $(,)?) => {
        $(
            if let Some(h) = $field {
                $vec.push(h);
            }
        )*
    };
}

#[macro_export]
macro_rules! must_have_namespace {
    ($element:expr, $namespace:expr) => {
        if !$element
            .tag_name()
            .namespace()
            .is_some_and(|ns| ns == $namespace)
        {
            return Err(xml::XmlError::XmlInvalidNamespace {
                expected: $namespace,
                found: $element.tag_name().namespace(),
            });
        }
    };
}

#[macro_export]
macro_rules! must_be_tag {
    ($element:expr, $tag:expr) => {
        if $element.tag_name().name() != $tag {
            return Err(xml::XmlError::XmlInvalidTag {
                expected: $tag,
                found: $element.tag_name().name(),
            });
        }
    };
}

#[macro_export]
macro_rules! must_be_element {
    ($element:expr) => {
        if !$element.is_element() {
            return Err(xml::XmlError::InvalidNodeType {
                expected: xml::parser::NodeType::Element,
                found: $element.node_type(),
            });
        }
    };
}

#[macro_export]
macro_rules! must_be_root {
    ($element:expr) => {
        if !$element.is_root() {
            return Err(xml::XmlError::InvalidNodeType {
                expected: xml::parser::NodeType::Root,
                found: $element.node_type(),
            });
        }
    };
}

#[macro_export]
macro_rules! must_be_text {
    ($element:expr) => {
        if !$element.is_text() {
            return Err(xml::XmlError::InvalidNodeType {
                expected: xml::parser::NodeType::Text,
                found: $element.node_type(),
            });
        }
    };
}

#[macro_export]
macro_rules! define_custom_tagname {
    ($name:ident, $tagName:expr, $namespace:expr) => {
        #[derive(Debug, Clone, PartialEq, Eq)]
        pub struct $name;

        impl crate::traits::TagName for $name {
            fn tag_name(&self) -> &'static str {
                $tagName
            }

            fn namespace(&self) -> Option<&'static str> {
                $namespace
            }
        }

        impl<'a> $name {
            pub fn new_tag<V>(value: V) -> Tag<'a, V, Self>
            where
                V: crate::traits::TagValue<'a>,
            {
                Tag::new(Self, value.into())
            }

            pub fn new_tag1<V, A>(value: V, attr: A) -> crate::traits::Tag1<'a, V, Self, A>
            where
                V: crate::traits::TagValue<'a>,
                A: crate::traits::Attribute<'a>,
            {
                crate::traits::Tag1::new(Self, value.into(), attr)
            }
        }
    };
}

#[macro_export]
macro_rules! define_tagname {
    ($name:ident, $namespace:expr) => {
        crate::define_custom_tagname!($name, stringify!($name), $namespace);
    };
}

#[macro_export]
macro_rules! define_tag {
    ($name:ident, $(($typ:ident, $field:ident)),+) => {


        #[derive(Debug, Clone)]
        pub struct $name<'a, V, N, $($typ),+>
        where
            V: TagValue<'a>,
            N: TagName,
            $($typ: Attribute<'a>,)+
        {
            pub name: N,
            pub value: V,
            $(pub $field: $typ,)+

            __phantom: std::marker::PhantomData<&'a V>,
        }

        impl<'a, V, N, $($typ),+> $name<'a, V, N, $($typ),+>
        where
            V: TagValue<'a>,
            N: TagName,
            $($typ: Attribute<'a>,)+
        {
            pub fn new(name: N, value: V, $($field: $typ),+) -> Self {
                Self {
                    name,
                    value,
                    $($field,)+
                    __phantom: std::marker::PhantomData,
                }
            }

            pub fn into_element(self) -> xml::builder::Element<'a> {
                let mut element = self.value.into_element(self.name.tag_name(),self.name.namespace());

                $(
                    if let Some(value) = self.$field.value()  {
                        element = element.add_attribute(xml::builder::Attribute::new_with_namespace(
                            self.$field.name(),
                            value,
                            self.$field.namespace(),
                        ));
                    }
                )+

                element
            }
        }

        impl<'a, V, N, $($typ),+> Into<xml::builder::Element<'a>> for $name<'a, V, N, $($typ),+>
        where
            V: TagValue<'a>,
            N: TagName,
            $($typ: Attribute<'a>,)+
        {
            fn into(self) -> xml::builder::Element<'a> {
                self.into_element()
            }
        }

        impl<'a, V, N, $($typ),+> TagValue<'a> for $name<'a, V, N, $($typ),+>
        where
            N: TagName,
            V: TagValue<'a>,
            $($typ: Attribute<'a>,)+
        {
            fn into_element(self, name: &'static str, namespace: Option<&'static str>) -> xml::builder::Element<'a> {
                let parent = xml::builder::Element::new(name)
                    .set_namespace_optional(namespace);

                let child = self.into_element();

                parent.add_child(child)
            }
        }
    };
}

#[macro_export]
macro_rules! push_element {
    ($vec:expr,[$($tag:expr),*]) => {

        $(
            if let Some(tag) = $tag {
                $vec.push(tag.into());
            }
        )*
    };
}
