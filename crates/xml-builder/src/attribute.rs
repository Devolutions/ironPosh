/// Represents an XML attribute with a name and value.
#[derive(Debug, Clone)]
pub struct Attribute<'a> {
    /// The name of the attribute.
    name: &'a str,
    /// The value of the attribute.
    value: &'a str,

    namespace: Option<crate::Namespace<'a>>,
}

impl<'a> Attribute<'a> {
    /// Creates a new instance of `Attribute`.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the attribute.
    /// * `value` - The value of the attribute.
    ///
    /// # Example
    ///
    /// ```
    /// use xml_builder::Attribute;
    /// let attribute = Attribute::new("name", "value");
    /// ```
    pub fn new(name: &'a str, value: &'a str) -> Self {
        Attribute {
            name,
            value,
            namespace: None,
        }
    }

    pub fn set_namespace(mut self, namespace: crate::Namespace<'a>) -> Self {
        self.namespace = Some(namespace);
        self
    }

    pub fn get_namespaces(
        &self,
        namespaces_set: &mut std::collections::HashSet<crate::Namespace<'a>>,
    ) {
        if let Some(namespace) = &self.namespace {
            namespaces_set.insert(namespace.clone());
        }
    }
}

impl std::fmt::Display for Attribute<'_> {
    /// Formats the attribute as a string in the format `name="value"`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(namespace) = &self.namespace {
            write!(f, "{}:{}", namespace.alias, self.name)?;
        } else {
            write!(f, "{}", self.name)?;
        }
        write!(f, "{}=\"{}\"", self.name, self.value)?;
        Ok(())
    }
}
