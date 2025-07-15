use std::fmt::Debug;
use xml::builder::Element;

use crate::traits::{Attribute, Tag1, Tag2, TagName, TagValue};

pub trait NamespaceWithAlias<'a> {
    const NAMESPACE: &'static str;
    const ALIAS: &'static str;
}

#[derive(Debug, Clone)]
pub struct PowerShellNamespaceAlias;

impl NamespaceWithAlias<'_> for PowerShellNamespaceAlias {
    const NAMESPACE: &'static str = "http://schemas.microsoft.com/powershell/Microsoft.PowerShell";
    const ALIAS: &'static str = "ps";
}

#[derive(Debug, Clone)]
pub struct RspShellNamespaceAlias;

impl NamespaceWithAlias<'_> for RspShellNamespaceAlias {
    const NAMESPACE: &'static str = "http://schemas.microsoft.com/wbem/wsman/1/windows/shell";
    const ALIAS: &'static str = "rsp";
}

#[derive(Debug, Clone)]
pub struct DeclareNamespaces<'a, N, T>
where
    T: Into<Element<'a>> + Debug + Clone,
    N: NamespaceWithAlias<'a>,
{
    tag: T,
    __phantom: std::marker::PhantomData<&'a T>,
    __phantom_namespace: std::marker::PhantomData<N>,
}

impl<'a, N, T> DeclareNamespaces<'a, N, T>
where
    T: Into<Element<'a>> + Debug + Clone,
    N: NamespaceWithAlias<'a>,
{
    pub fn new(tag: T) -> Self {
        Self {
            tag,
            __phantom: std::marker::PhantomData,
            __phantom_namespace: std::marker::PhantomData,
        }
    }
}

impl<'a, N, T> From<T> for DeclareNamespaces<'a, N, T>
where
    T: Into<Element<'a>> + Debug + Clone,
    N: NamespaceWithAlias<'a>,
{
    fn from(tag: T) -> Self {
        Self::new(tag)
    }
}

impl<'a, N, T> AsRef<T> for DeclareNamespaces<'a, N, T>
where
    T: Into<Element<'a>> + Debug + Clone,
    N: NamespaceWithAlias<'a>,
{
    fn as_ref(&self) -> &T {
        &self.tag
    }
}

impl<'a, T, N> Into<Element<'a>> for DeclareNamespaces<'a, N, T>
where
    T: Into<Element<'a>> + Debug + Clone,
    N: NamespaceWithAlias<'a>,
{
    fn into(self) -> Element<'a> {
        let mut element = self.tag.into();
        element = element.add_namespace_alias(N::NAMESPACE, N::ALIAS);
        element
    }
}

impl<'a, V, N, A, NS> From<(V, A)> for DeclareNamespaces<'a, NS, Tag1<'a, V, N, A>>
where
    A: Attribute<'a> + Debug + Clone,
    V: TagValue<'a> + Debug + Clone,
    N: TagName + Debug + Clone,
    NS: NamespaceWithAlias<'a> + Debug + Clone,
{
    fn from(value: (V, A)) -> Self {
        let tag = Tag1::from(value);
        DeclareNamespaces::new(tag)
    }
}

impl<'a, V, N, A, A1, NS> From<(V, A, A1)> for DeclareNamespaces<'a, NS, Tag2<'a, V, N, A, A1>>
where
    A: Attribute<'a> + Debug + Clone,
    A1: Attribute<'a> + Debug + Clone,
    V: TagValue<'a> + Debug + Clone,
    N: TagName + Debug + Clone,
    NS: NamespaceWithAlias<'a> + Debug + Clone,
{
    fn from(value: (V, A, A1)) -> Self {
        let (v, a, a1) = value;
        let tag = Tag2::new(v, a, a1);
        DeclareNamespaces::new(tag)
    }
}
