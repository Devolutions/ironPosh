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

        impl<'a> TryFrom<&ironposh_xml::parser::Namespace<'a>> for Namespace {
            type Error = &'static str;
            #[inline]
            fn try_from(ns: &ironposh_xml::parser::Namespace<'a>) -> Result<Self, Self::Error> {
                Self::try_from(ns.uri())
            }
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
    WsEventing2004    => { alias: Some("e")   , uri: "http://schemas.xmlsoap.org/ws/2004/08/eventing" },
    WsEnumeration2004 => { alias: Some("n")   , uri: "http://schemas.xmlsoap.org/ws/2004/09/enumeration" },
    WsmanFault        => { alias: Some("f")   , uri: "http://schemas.microsoft.com/wbem/wsman/1/wsmanfault" },
    PowerShellRemoting=> { alias: None        , uri: "http://schemas.microsoft.com/powershell" },
    XmlSchemaInstance => { alias: Some("xsi") , uri: "http://www.w3.org/2001/XMLSchema-instance" },
}

// -----------------------------------------------------------------------------
//                   OPTIONAL GROUPING / DECLARATION TYPES
// -----------------------------------------------------------------------------
#[derive(Debug, Clone, Default)]
pub struct NamespaceDeclaration(Vec<Namespace>);

impl NamespaceDeclaration {
    pub fn new() -> Self {
        Self::default()
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

impl<'a> ironposh_xml::mapping::FromXml<'a> for NamespaceDeclaration {
    /// Captures the namespaces declared on this element. Declarations we don't
    /// recognize are skipped — they don't affect matching, which compares URIs.
    fn from_xml(node: ironposh_xml::parser::Node<'a, 'a>) -> Result<Self, ironposh_xml::XmlError> {
        let namespaces = node
            .namespaces()
            .filter_map(|ns| Namespace::try_from(ns).ok())
            .collect();
        Ok(NamespaceDeclaration(namespaces))
    }
}
