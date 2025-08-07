use std::fmt::Debug;

// -----------------------------------------------------------------------------
//                               THE MACRO
// -----------------------------------------------------------------------------
#[macro_export]
macro_rules! define_namespaces {
    // public interface ────────────────────────────────────────────────────────
    ( $( $variant:ident => { alias: $alias:expr, uri: $uri:expr } ),+ $(,)? ) => {
        // ---------- enum -----------------------------------------------------
        #[derive(Debug, Clone, Eq, PartialEq)]
        pub enum Namespace {
            $( $variant, )+
        }

        // ---------- core helpers --------------------------------------------
        impl Namespace {
            /// `(uri, alias)`
            #[inline]
            pub const fn as_tuple(&self) -> (&'static str, Option<&'static str>) {
                match self {
                    $( Self::$variant => ($uri, $alias), )+
                }
            }

            #[inline] pub const fn uri(&self)   -> &'static str         { self.as_tuple().0 }
            #[inline] pub const fn alias(&self) -> Option<&'static str> { self.as_tuple().1 }
        }

        // ---------- conversions ---------------------------------------------
        impl TryFrom<&str> for Namespace {
            type Error = &'static str;
            fn try_from(value: &str) -> Result<Self, Self::Error> {
                match value {
                    $( $uri => Ok(Self::$variant), )+
                    _ => Err("Unknown namespace"),
                }
            }
        }

        impl<'a> TryFrom<&xml::parser::Namespace<'a>> for Namespace {
            type Error = &'static str;
            #[inline]
            fn try_from(ns: &xml::parser::Namespace<'a>) -> Result<Self, Self::Error> {
                Self::try_from(ns.uri())
            }
        }

        // ---------- XmlDeserialize support -----------------------------------
        pub struct NamespaceVisitor { namespace: Option<Namespace> }

        impl<'a> xml::parser::XmlVisitor<'a> for NamespaceVisitor {
            type Value = Namespace;

            fn visit_children(
                &mut self,
                _children: impl Iterator<Item = xml::parser::Node<'a, 'a>>,
            ) -> Result<(), xml::XmlError> { Ok(()) }

            fn visit_node(
                &mut self,
                node: xml::parser::Node<'a, 'a>,
            ) -> Result<(), xml::XmlError> {
                let Some(ns) = node.tag_name().namespace() else {
                    return Err(xml::XmlError::InvalidXml("No namespace found".into()));
                };
                self.namespace = Some(
                    Namespace::try_from(ns)
                        .map_err(|_| xml::XmlError::InvalidXml(format!("Unknown namespace: {ns}")))?,
                );
                Ok(())
            }

            fn finish(self) -> Result<Self::Value, xml::XmlError> {
                self.namespace.ok_or_else(|| xml::XmlError::InvalidXml("No namespace found".into()))
            }
        }

        impl<'a> xml::parser::XmlDeserialize<'a> for Namespace {
            type Visitor = NamespaceVisitor;
            #[inline] fn visitor() -> Self::Visitor { NamespaceVisitor { namespace: None } }
        }
    };
}

// -----------------------------------------------------------------------------
//                           MACRO INVOCATION
// -----------------------------------------------------------------------------
define_namespaces! {
    WsmanShell        => { alias: Some("rsp") , uri: "http://schemas.microsoft.com/wbem/wsman/1/windows/shell" },
    WsAddressing2004  => { alias: Some("a")   , uri: "http://schemas.xmlsoap.org/ws/2004/08/addressing" },
    SoapEnvelope2003  => { alias: Some("s")   , uri: "http://www.w3.org/2003/05/soap-envelope" },
    MsWsmanSchema     => { alias: Some("p")   , uri: "http://schemas.microsoft.com/wbem/wsman/1/wsman.xsd" },
    DmtfWsmanSchema   => { alias: Some("w")   , uri: "http://schemas.dmtf.org/wbem/wsman/1/wsman.xsd" },
    WsTransfer2004    => { alias: Some("x")   , uri: "http://schemas.xmlsoap.org/ws/2004/09/transfer" },
    PowerShellRemoting=> { alias: None        , uri: "http://schemas.microsoft.com/powershell" },
    XmlSchemaInstance => { alias: Some("xsi") , uri: "http://www.w3.org/2001/XMLSchema-instance" },
}

// -----------------------------------------------------------------------------
//                   OPTIONAL GROUPING / DECLARATION TYPES
// -----------------------------------------------------------------------------
#[derive(Debug, Clone)]
pub struct NamespaceDeclaration(Vec<Namespace>);

impl Default for NamespaceDeclaration {
    fn default() -> Self {
        Self::new()
    }
}
impl NamespaceDeclaration {
    pub fn new() -> Self {
        Self(Vec::new())
    }
    pub fn namespaces(&self) -> &[Namespace] {
        &self.0
    }
    pub fn push(&mut self, ns: Namespace) {
        self.0.push(ns);
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
        _c: impl Iterator<Item = xml::parser::Node<'a, 'a>>,
    ) -> Result<(), xml::XmlError> {
        Ok(())
    }

    fn visit_node(&mut self, node: xml::parser::Node<'a, 'a>) -> Result<(), xml::XmlError> {
        for ns in node.namespaces() {
            self.namespaces.push(
                Namespace::try_from(ns).map_err(|_| {
                    xml::XmlError::InvalidXml(format!("Unknown namespace: {:?}", ns))
                })?,
            );
        }
        Ok(())
    }

    fn finish(self) -> Result<Self::Value, xml::XmlError> {
        Ok(NamespaceDeclaration(self.namespaces))
    }
}

impl<'a> xml::parser::XmlDeserialize<'a> for NamespaceDeclaration {
    type Visitor = NamespaceDeclarationVisitor;
    #[inline]
    fn visitor() -> Self::Visitor {
        NamespaceDeclarationVisitor {
            namespaces: Vec::new(),
        }
    }
    fn from_node(node: xml::parser::Node<'a, 'a>) -> Result<Self, xml::XmlError> {
        xml::parser::NodeDeserializer::new(node).deserialize(Self::visitor())
    }
}
