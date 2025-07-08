use crate::{Declaration, RootElement};

/// Represents a builder for constructing an XML document.
pub struct Builder<'a> {
    /// The XML declaration.
    declaration: Option<Declaration<'a>>,
    /// The root element of the XML document.
    element: RootElement<'a>,
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
    /// use xml_builder::{Builder, Declaration, Element, RootElement};
    /// let declaration = Declaration::new("1.0", "UTF-8").with_standalone(true);
    /// let element = Element::new("root");
    /// let root_element = RootElement::new(element);
    /// let builder = Builder::new(Some(declaration), root_element);
    /// ```
    pub fn new(declaration: Option<Declaration<'a>>, element: RootElement<'a>) -> Self {
        Builder {
            declaration,
            element,
        }
    }
}

impl<'a> std::fmt::Display for Builder<'a> {
    /// Formats the builder and its content as an XML document string.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(declaration) = &self.declaration {
            write!(f, "{declaration}")?;
        }
        write!(f, "{}", self.element)?;
        Ok(())
    }
}
