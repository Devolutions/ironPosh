use std::fmt::Debug;
use xml::parser::XmlDeserialize;

pub const PWSH_NAMESPACE: &str = "http://schemas.microsoft.com/wbem/wsman/1/windows/shell";
pub const PWSH_NAMESPACE_ALIAS: &str = "rsp";

pub const WSA_NAMESPACE: &str = "http://schemas.xmlsoap.org/ws/2004/08/addressing";
pub const WSA_NAMESPACE_ALIAS: &str = "a";

pub const SOAP_NAMESPACE: &str = "http://www.w3.org/2003/05/soap-envelope";
pub const SOAP_NAMESPACE_ALIAS: &str = "s";

pub const WSMAN_NAMESPACE: &str = "http://schemas.microsoft.com/wbem/wsman/1/wsman.xsd";
pub const WSMAN_NAMESPACE_ALIAS: &str = "w";

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Namespace {
    PowerShell,
    RspShell,
    WsAddressing,
    WsManagementHeader,
    Soap,
}

impl Namespace {
    pub fn as_tuple(&self) -> (&'static str, &'static str) {
        match self {
            Namespace::PowerShell => (PWSH_NAMESPACE, PWSH_NAMESPACE_ALIAS),
            Namespace::RspShell => (
                "http://schemas.microsoft.com/wbem/wsman/1/windows/shell",
                "rsp",
            ),
            Namespace::WsAddressing => (WSA_NAMESPACE, WSA_NAMESPACE_ALIAS),
            Namespace::WsManagementHeader => (WSMAN_NAMESPACE, WSMAN_NAMESPACE_ALIAS),
            Namespace::Soap => (SOAP_NAMESPACE, SOAP_NAMESPACE_ALIAS),
        }
    }

    pub fn url(&self) -> &'static str {
        self.as_tuple().0
    }

    pub fn alias(&self) -> &'static str {
        self.as_tuple().1
    }
}

impl<'a> XmlDeserialize<'a> for Namespace {
    type Visitor = NamespaceVisitor;

    fn visitor() -> Self::Visitor {
        NamespaceVisitor { namespace: None }
    }
}

impl TryFrom<&str> for Namespace {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "http://schemas.microsoft.com/powershell/Microsoft.PowerShell" => {
                Ok(Namespace::PowerShell)
            }
            "http://schemas.microsoft.com/wbem/wsman/1/windows/shell" => Ok(Namespace::RspShell),
            "http://www.w3.org/2005/08/addressing" => Ok(Namespace::WsAddressing),
            "http://schemas.dmtf.org/wbem/wsman/1/wsman.xsd" => Ok(Namespace::WsManagementHeader),
            _ => Err("Unknown namespace"),
        }
    }
}

pub struct NamespaceVisitor {
    namespace: Option<Namespace>,
}

impl<'a> xml::parser::XmlVisitor<'a> for NamespaceVisitor {
    type Value = Namespace;

    fn visit_children(
        &mut self,
        _children: impl Iterator<Item = xml::parser::Node<'a, 'a>>,
    ) -> Result<(), xml::XmlError<'a>> {
        Ok(())
    }

    fn visit_node(&mut self, node: xml::parser::Node<'a, 'a>) -> Result<(), xml::XmlError<'a>> {
        let Some(namespace) = node.tag_name().namespace() else {
            return Err(xml::XmlError::InvalidXml("No namespace found".to_string()));
        };

        match Namespace::try_from(namespace) {
            Ok(ns) => {
                self.namespace = Some(ns);
            }
            Err(_) => {
                return Err(xml::XmlError::InvalidXml(format!(
                    "Unknown namespace: {namespace}"
                )));
            }
        };

        Ok(())
    }

    fn finish(self) -> Result<Self::Value, xml::XmlError<'a>> {
        self.namespace
            .ok_or(xml::XmlError::InvalidXml("No namespace found".to_string()))
    }
}

#[derive(Debug, Clone)]
pub struct NamespaceDeclaration(Vec<Namespace>);

impl Default for NamespaceDeclaration {
    fn default() -> Self {
        Self::new()
    }
}

impl NamespaceDeclaration {
    pub fn new() -> Self {
        NamespaceDeclaration(Vec::new())
    }

    pub fn namespaces(&self) -> &[Namespace] {
        &self.0
    }

    pub fn push(&mut self, namespace: Namespace) {
        self.0.push(namespace);
    }
}

pub struct NamespaceDeclarationVisitor {
    namespaces: Vec<Namespace>,
}

impl<'a> xml::parser::XmlVisitor<'a> for NamespaceDeclarationVisitor {
    type Value = NamespaceDeclaration;

    fn visit_children(
        &mut self,
        _children: impl Iterator<Item = xml::parser::Node<'a, 'a>>,
    ) -> Result<(), xml::XmlError<'a>> {
        Ok(())
    }

    fn visit_node(&mut self, _node: xml::parser::Node<'a, 'a>) -> Result<(), xml::XmlError<'a>> {
        todo!()
    }

    fn finish(self) -> Result<Self::Value, xml::XmlError<'a>> {
        Ok(NamespaceDeclaration(self.namespaces))
    }
}

impl<'a> XmlDeserialize<'a> for NamespaceDeclaration {
    type Visitor = NamespaceDeclarationVisitor;

    fn visitor() -> Self::Visitor {
        NamespaceDeclarationVisitor {
            namespaces: Vec::new(),
        }
    }

    fn from_node(node: xml::parser::Node<'a, 'a>) -> Result<Self, xml::XmlError<'a>> {
        xml::parser::NodeDeserializer::new(node).deserialize(Self::visitor())
    }
}
