use std::fmt::Debug;
use xml::builder::Element;

pub trait NamespaceAliasTuple<'a> {
    fn namespace_alias_tuple(&self) -> (&'static str, &'static str);
}

#[derive(Debug, Clone)]
pub struct PowerShellNamespaceAlias;

impl NamespaceAliasTuple<'_> for PowerShellNamespaceAlias {
    fn namespace_alias_tuple(&self) -> (&'static str, &'static str) {
        (
            "http://schemas.microsoft.com/powershell/Microsoft.PowerShell",
            "ps",
        )
    }
}

#[derive(Debug, Clone)]
pub struct DeclareNamespaces<'a, N, T>
where
    T: Into<Element<'a>> + Debug + Clone,
    N: NamespaceAliasTuple<'a>,
{
    tag: T,
    namespace_alias: Option<N>,
    __phantom: std::marker::PhantomData<&'a T>,
}

impl<'a, N, T> DeclareNamespaces<'a, N, T>
where
    T: Into<Element<'a>> + Debug + Clone,
    N: NamespaceAliasTuple<'a>,
{
    pub fn new(tag: T) -> Self {
        Self {
            tag,
            namespace_alias: None,
            __phantom: std::marker::PhantomData,
        }
    }

    pub fn with_namespace_alias(mut self, namespace_alias: N) -> Self {
        self.namespace_alias = Some(namespace_alias);
        self
    }
}

impl<'a, N, T> AsRef<T> for DeclareNamespaces<'a, N, T>
where
    T: Into<Element<'a>> + Debug + Clone,
    N: NamespaceAliasTuple<'a>,
{
    fn as_ref(&self) -> &T {
        &self.tag
    }
}

impl<'a, T, N> Into<Element<'a>> for DeclareNamespaces<'a, N, T>
where
    T: Into<Element<'a>> + Debug + Clone,
    N: NamespaceAliasTuple<'a>,
{
    fn into(self) -> Element<'a> {
        let mut element = self.tag.into();
        if let Some(namespace_alias) = &self.namespace_alias {
            let (namespace, alias) = namespace_alias.namespace_alias_tuple();
            element = element.add_namespace_alias(namespace, alias);
        }
        element
    }
}
