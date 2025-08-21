use crate::builder::{Declaration, Element, NamespaceWrite, XmlBuilderError};

/// Represents a builder for constructing an XML document.
pub struct Builder<'a> {
    /// The XML declaration.
    declaration: Option<Declaration<'a>>,
    /// The root element of the XML document.
    element: Element<'a>,
}

impl<'a> Builder<'a> {
    /// Creates a new instance of `Builder` with the given declaration and root element.
    ///
    /// # Arguments
    ///
    /// * `declaration` - The XML declaration.
    /// * `element` - The root element of the XML document.
    ///
    /// # Example
    ///
    /// ```
    /// use xml::builder::{Builder, Declaration, Element};
    /// let declaration = Declaration::new("1.0", "UTF-8").with_standalone(true);
    /// let element = Element::new("root");
    /// let builder = Builder::new(Some(declaration), element);
    /// ```
    pub fn new(declaration: Option<Declaration<'a>>, element: Element<'a>) -> Self {
        Builder {
            declaration,
            element,
        }
    }

    pub fn write_to<W: std::io::Write>(&self, mut w: W) -> Result<(), XmlBuilderError> {
        if let Some(decl) = &self.declaration {
            decl.write(&mut w)?; // converts to XmlError via From
            w.write_all(b" \n")?;
        }
        self.element.ns_write(&mut w, None)
    }

    pub fn to_xml_string(&self) -> Result<String, XmlBuilderError> {
        let mut buf = Vec::new();
        self.write_to(&mut buf)?;
        Ok(String::from_utf8(buf).expect("XML must be UTF-8"))
    }
}

