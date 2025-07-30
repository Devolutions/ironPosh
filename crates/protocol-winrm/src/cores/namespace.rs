use std::fmt::Debug;
use xml::parser::XmlDeserialize;

pub const WSMAN_SHELL_NAMESPACE_URI: &str =
    "http://schemas.microsoft.com/wbem/wsman/1/windows/shell";
pub const WSMAN_SHELL_NAMESPACE_ALIAS: &str = "rsp";

pub const POWERSHELL_REMOTING_NAMESPACE_URI: &str =
    "http://schemas.microsoft.com/powershell/Microsoft.PowerShell";

pub const WS_ADDRESSING_NAMESPACE_URI: &str = "http://schemas.xmlsoap.org/ws/2004/08/addressing";
pub const WS_ADDRESSING_NAMESPACE_ALIAS: &str = "a";

pub const SOAP_ENVELOPE_NAMESPACE_URI: &str = "http://www.w3.org/2003/05/soap-envelope";
pub const SOAP_ENVELOPE_NAMESPACE_ALIAS: &str = "s";

pub const MS_WSMAN_SCHEMA_NAMESPACE_URI: &str =
    "http://schemas.microsoft.com/wbem/wsman/1/wsman.xsd";
pub const MS_WSMAN_SCHEMA_NAMESPACE_ALIAS: &str = "p";

pub const DMTF_WSMAN_SCHEMA_NAMESPACE_URI: &str = "http://schemas.dmtf.org/wbem/wsman/1/wsman.xsd";
pub const DMTF_WSMAN_SCHEMA_NAMESPACE_ALIAS: &str = "w";

pub const WS_TRANSFER_NAMESPACE_URI: &str = "http://schemas.xmlsoap.org/ws/2004/09/transfer";
pub const WS_TRANSFER_NAMESPACE_ALIAS: &str = "x";

pub const XML_SCHEMA_INSTANCE_NAMESPACE_URI: &str = "http://www.w3.org/2001/XMLSchema-instance";
pub const XML_SCHEMA_INSTANCE_NAMESPACE_ALIAS: &str = "xsi";

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Namespace {
    WsmanShell,
    WsAddressing2004,
    MsWsmanSchema,
    DmtfWsmanSchema,
    SoapEnvelope2003,
    WsTransfer2004,
    PowerShellRemoting,
    XmlSchemaInstance,
}

impl Namespace {
    pub fn as_tuple(&self) -> (&'static str, Option<&'static str>) {
        match self {
            Namespace::WsmanShell => (WSMAN_SHELL_NAMESPACE_URI, Some(WSMAN_SHELL_NAMESPACE_ALIAS)),
            Namespace::WsAddressing2004 => (
                WS_ADDRESSING_NAMESPACE_URI,
                Some(WS_ADDRESSING_NAMESPACE_ALIAS),
            ),
            Namespace::MsWsmanSchema => (
                MS_WSMAN_SCHEMA_NAMESPACE_URI,
                Some(MS_WSMAN_SCHEMA_NAMESPACE_ALIAS),
            ),
            Namespace::DmtfWsmanSchema => (
                DMTF_WSMAN_SCHEMA_NAMESPACE_URI,
                Some(DMTF_WSMAN_SCHEMA_NAMESPACE_ALIAS),
            ),
            Namespace::SoapEnvelope2003 => (
                SOAP_ENVELOPE_NAMESPACE_URI,
                Some(SOAP_ENVELOPE_NAMESPACE_ALIAS),
            ),
            Namespace::WsTransfer2004 => {
                (WS_TRANSFER_NAMESPACE_URI, Some(WS_TRANSFER_NAMESPACE_ALIAS))
            }
            Namespace::PowerShellRemoting => (POWERSHELL_REMOTING_NAMESPACE_URI, None),
            Namespace::XmlSchemaInstance => (
                XML_SCHEMA_INSTANCE_NAMESPACE_URI,
                Some(XML_SCHEMA_INSTANCE_NAMESPACE_ALIAS),
            ),
        }
    }

    pub fn url(&self) -> &'static str {
        self.as_tuple().0
    }

    pub fn alias(&self) -> Option<&'static str> {
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
            POWERSHELL_REMOTING_NAMESPACE_URI => Ok(Namespace::PowerShellRemoting),
            WSMAN_SHELL_NAMESPACE_URI => Ok(Namespace::WsmanShell),
            WS_ADDRESSING_NAMESPACE_URI => Ok(Namespace::WsAddressing2004),
            MS_WSMAN_SCHEMA_NAMESPACE_URI => Ok(Namespace::MsWsmanSchema),
            DMTF_WSMAN_SCHEMA_NAMESPACE_URI => Ok(Namespace::DmtfWsmanSchema),
            SOAP_ENVELOPE_NAMESPACE_URI => Ok(Namespace::SoapEnvelope2003),
            WS_TRANSFER_NAMESPACE_URI => Ok(Namespace::WsTransfer2004),
            XML_SCHEMA_INSTANCE_NAMESPACE_URI => Ok(Namespace::XmlSchemaInstance),
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
    ) -> Result<(), xml::XmlError> {
        Ok(())
    }

    fn visit_node(&mut self, node: xml::parser::Node<'a, 'a>) -> Result<(), xml::XmlError> {
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

    fn finish(self) -> Result<Self::Value, xml::XmlError> {
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

impl IntoIterator for NamespaceDeclaration {
    type Item = Namespace;
    type IntoIter = std::vec::IntoIter<Namespace>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
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
    ) -> Result<(), xml::XmlError> {
        Ok(())
    }

    fn visit_node(&mut self, node: xml::parser::Node<'a, 'a>) -> Result<(), xml::XmlError> {
        let namespaces = node.namespaces();
        for namespace in namespaces {
            match Namespace::try_from(namespace) {
                Ok(ns) => self.namespaces.push(ns),
                Err(_) => {
                    return Err(xml::XmlError::InvalidXml(format!(
                        "Unknown namespace: {namespace:?}"
                    )));
                }
            }
        }
        Ok(())
    }

    fn finish(self) -> Result<Self::Value, xml::XmlError> {
        Ok(NamespaceDeclaration(self.namespaces))
    }
}

impl<'a> TryFrom<&xml::parser::Namespace<'a>> for Namespace {
    type Error = &'static str;

    fn try_from(namespace: &xml::parser::Namespace<'a>) -> Result<Self, Self::Error> {
        Self::try_from(namespace.uri()).or_else(|_| Self::try_from(namespace.uri()))
    }
}

impl<'a> XmlDeserialize<'a> for NamespaceDeclaration {
    type Visitor = NamespaceDeclarationVisitor;

    fn visitor() -> Self::Visitor {
        NamespaceDeclarationVisitor {
            namespaces: Vec::new(),
        }
    }

    fn from_node(node: xml::parser::Node<'a, 'a>) -> Result<Self, xml::XmlError> {
        xml::parser::NodeDeserializer::new(node).deserialize(Self::visitor())
    }
}
