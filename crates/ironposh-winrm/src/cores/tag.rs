use ironposh_xml::builder::Element;
use ironposh_xml::mapping::{FromXml, NodeExt};

use crate::cores::WsUuid;
use crate::cores::namespace::{Namespace, NamespaceDeclaration};
use crate::cores::tag_value::{Text, U32};
use crate::impl_tag_from;

use super::attribute::Attribute;
use super::tag_name::TagName;
use super::tag_value::TagValue;

#[derive(Debug, Clone)]
pub struct Tag<'a, V, N>
where
    V: TagValue<'a>,
    N: TagName,
{
    pub value: V,
    pub attributes: Vec<Attribute<'a>>,
    /// The namespaces are the declaration of namespaces used in this tag.
    /// For example
    /// <s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/">
    /// would have a namespace declaration for "s" with the URI "http://schemas.xmlsoap.org/soap/envelope/".
    pub namespaces_declaration: NamespaceDeclaration,

    __phantom: std::marker::PhantomData<&'a V>,
    __phantom_name: std::marker::PhantomData<N>,
}

pub struct TagNameHolder<'a, N, V>
where
    N: TagName,
    V: TagValue<'a>,
{
    name: N,
    attributes: Option<Vec<Attribute<'a>>>,
    namespaces_declaration: NamespaceDeclaration,
    __phantom: std::marker::PhantomData<&'a V>,
}

impl<'a, N, V> TagNameHolder<'a, N, V>
where
    N: TagName,
    V: TagValue<'a>,
{
    pub fn with_value(self, value: V) -> Tag<'a, V, N> {
        let mut tag = Tag::new(value).with_name(self.name);

        if let Some(attrs) = self.attributes {
            for attr in attrs {
                tag = tag.with_attribute(attr);
            }
        }

        for declaration in self.namespaces_declaration {
            tag = tag.with_declaration(declaration);
        }

        tag
    }

    pub fn with_attribute(mut self, attribute: Attribute<'a>) -> Self {
        if let Some(ref mut attrs) = self.attributes {
            attrs.push(attribute);
        } else {
            self.attributes = Some(vec![attribute]);
        }
        self
    }

    pub fn with_declaration(mut self, declaration: Namespace) -> Self {
        self.namespaces_declaration.push(declaration);
        self
    }
}

impl<'a, V, N> Tag<'a, V, N>
where
    V: TagValue<'a>,
    N: TagName,
{
    pub fn new(value: impl Into<V>) -> Self {
        Self {
            value: value.into(),
            attributes: Vec::new(),
            namespaces_declaration: NamespaceDeclaration::new(),
            __phantom: std::marker::PhantomData,
            __phantom_name: std::marker::PhantomData,
        }
    }

    pub fn from_name(name: N) -> TagNameHolder<'a, N, V>
    where
        N: TagName,
    {
        TagNameHolder {
            name,
            attributes: None,
            namespaces_declaration: NamespaceDeclaration::new(),
            __phantom: std::marker::PhantomData,
        }
    }

    /// Does not do anything, just returns self.
    /// This is useful for compiler to infer the type of `N` when using `Tag::new`.
    pub fn with_name(self, _name: N) -> Self {
        self
    }

    pub fn with_attribute(mut self, attribute: Attribute<'a>) -> Self {
        self.attributes.push(attribute);
        self
    }

    pub fn with_declaration(mut self, declaration: Namespace) -> Self {
        self.namespaces_declaration.push(declaration);
        self
    }

    pub fn into_element(self) -> Element<'a> {
        let mut element = Element::new(N::TAG_NAME);
        if let Some(ns) = N::NAMESPACE {
            element = element.set_namespace(ns);
        }

        // Add namespace declarations to the element
        for namespace in self.namespaces_declaration.namespaces() {
            let (url, alias) = namespace.as_tuple();
            element = element.add_namespace_declaration(url, alias);
        }

        for attribute in self.attributes {
            element = element.add_attribute(attribute.into());
        }

        self.value.append_to_element(element)
    }

    pub fn name(&self) -> &'static str {
        N::TAG_NAME
    }

    pub fn clone_value(&self) -> V
    where
        V: Clone,
    {
        self.value.clone()
    }
}

impl<'a, V, N> From<V> for Tag<'a, V, N>
where
    V: TagValue<'a>,
    N: TagName + 'a,
{
    fn from(value: V) -> Self {
        Tag::new(value)
    }
}

impl<'a, V, N> FromXml<'a> for Tag<'a, V, N>
where
    V: TagValue<'a> + FromXml<'a>,
    N: TagName,
{
    fn from_xml(node: ironposh_xml::parser::Node<'a, 'a>) -> Result<Self, ironposh_xml::XmlError> {
        // Identity is the (namespace-URI, local-name) pair; the prefix is never
        // consulted. Usually a dispatcher already matched this element, so `node`
        // *is* this tag. When a parent tag carries another tag as its value
        // (`Tag<Tag<..>, _>`), we're handed the parent instead — descend to the
        // single N-named child.
        let element = if node.is_element_named(N::NAMESPACE, N::TAG_NAME) {
            node
        } else {
            // Wrapper case (`Tag<Tag<..>, _>`): exactly one child element, which
            // must be N. Reject zero (wrong wrapper), >1 (malformed), or a single
            // child of the wrong name rather than silently picking one.
            let mut elements = node
                .children()
                .filter(ironposh_xml::parser::Node::is_element);
            let only = elements
                .next()
                .ok_or_else(|| ironposh_xml::XmlError::XmlInvalidTag {
                    expected: N::TAG_NAME.to_string(),
                    found: node.tag_name().name().to_string(),
                })?;
            if elements.next().is_some() {
                return Err(ironposh_xml::XmlError::InvalidXml(format!(
                    "expected exactly one child element in <{}>",
                    node.tag_name().name()
                )));
            }
            if !only.is_element_named(N::NAMESPACE, N::TAG_NAME) {
                return Err(ironposh_xml::XmlError::XmlInvalidTag {
                    expected: N::TAG_NAME.to_string(),
                    found: only.tag_name().name().to_string(),
                });
            }
            only
        };

        let value = V::from_xml(element)?;
        let attributes = element
            .attributes()
            .filter_map(|attr| {
                Attribute::from_name_and_value(attr.name(), attr.value())
                    .ok()
                    .flatten()
            })
            .collect();
        let namespaces_declaration = NamespaceDeclaration::from_xml(element)?;

        Ok(Tag {
            value,
            attributes,
            namespaces_declaration,
            __phantom: std::marker::PhantomData,
            __phantom_name: std::marker::PhantomData,
        })
    }
}

/// A tag type's XML identity (name + namespace) exposed at the type level.
///
/// `Tag<'a, V, N>` forwards to its `N: TagName`. Reading identity through this
/// trait — rather than naming `N` syntactically — lets `#[derive(FromXml)]`
/// work through type aliases like `pub type Get<'a> = Tag<'a, Text<'a>, GetTag>`.
pub trait NamedTag {
    const TAG_NAME: &'static str;
    const NAMESPACE: Option<&'static str>;
}

impl<'a, V, N> NamedTag for Tag<'a, V, N>
where
    V: TagValue<'a>,
    N: TagName,
{
    const TAG_NAME: &'static str = N::TAG_NAME;
    const NAMESPACE: Option<&'static str> = N::NAMESPACE;
}

impl<'a, V, N> AsRef<V> for Tag<'a, V, N>
where
    V: TagValue<'a>,
    N: TagName,
{
    fn as_ref(&self) -> &V {
        &self.value
    }
}

impl<'a, V, N> From<Tag<'a, V, N>> for Element<'a>
where
    V: TagValue<'a>,
    N: TagName,
{
    fn from(val: Tag<'a, V, N>) -> Self {
        val.into_element()
    }
}

impl<'a, V, N> TagValue<'a> for Tag<'a, V, N>
where
    V: TagValue<'a>,
    N: TagName,
{
    fn append_to_element(self, element: Element<'a>) -> Element<'a> {
        let inner_element = self.into_element();
        element.add_child(inner_element)
    }
}

impl_tag_from!(&'a str => Tag<'a, Text<'a>, N>);
impl_tag_from!(String => Tag<'a, Text<'a>, N>);
impl_tag_from!(u32 => Tag<'a, U32, N>);
impl_tag_from!(uuid::Uuid => Tag<'a, WsUuid, N>);

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cores::CommandResponse;
    use ironposh_xml::parser::parse;

    const RSP: &str = "http://schemas.microsoft.com/wbem/wsman/1/windows/shell";

    /// A `Tag` whose value is itself a `Tag` (`<CommandResponse>` wrapping a
    /// `<CommandId>` child). `from_xml` must descend to the named child rather
    /// than parse the wrapper as the inner tag. Regression for the SSPI e2e.
    #[test]
    fn nested_tag_value_descends_to_child() {
        let uuid = "2D6534D0-6B12-40E3-B773-CBA26459CFA8";
        let xml = format!(
            r#"<rsp:CommandResponse xmlns:rsp="{RSP}"><rsp:CommandId>{uuid}</rsp:CommandId></rsp:CommandResponse>"#
        );
        let doc = parse(&xml).unwrap();
        let tag = CommandResponse::from_xml(doc.root_element())
            .expect("nested CommandResponse/CommandId should parse");
        assert_eq!(tag.value.value.0.to_string().to_uppercase(), uuid);
    }
}
