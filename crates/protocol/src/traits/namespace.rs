use std::fmt::Debug;
use xml::builder::Element;

pub trait NamespaceAliasTuple<'a> {
    fn namespace_alias_tuple() -> (&'static str, &'static str);
}

#[derive(Debug, Clone)]
pub struct PowerShellNamespaceAlias;

impl NamespaceAliasTuple<'_> for PowerShellNamespaceAlias {
    fn namespace_alias_tuple() -> (&'static str, &'static str) {
        (
            "http://schemas.microsoft.com/powershell/Microsoft.PowerShell",
            "ps",
        )
    }
}

#[derive(Debug, Clone)]
pub struct RspShellNamespaceAlias;

impl NamespaceAliasTuple<'_> for RspShellNamespaceAlias {
    fn namespace_alias_tuple() -> (&'static str, &'static str) {
        (
            "http://schemas.microsoft.com/wbem/wsman/1/windows/shell",
            "rsp",
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
    // namespace_alias: Option<N>,
    __phantom: std::marker::PhantomData<&'a T>,
    __phantom_namespace: std::marker::PhantomData<N>,
}

impl<'a, N, T> DeclareNamespaces<'a, N, T>
where
    T: Into<Element<'a>> + Debug + Clone,
    N: NamespaceAliasTuple<'a>,
{
    pub fn new(tag: T) -> Self {
        Self {
            tag,
            __phantom: std::marker::PhantomData,
            __phantom_namespace: std::marker::PhantomData,
        }
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
        let (namespace, alias) = N::namespace_alias_tuple();
        element = element.add_namespace_alias(namespace, alias);
        element
    }
}
