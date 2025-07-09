use std::collections::HashMap;

/// Represents an XML attribute with a name and value.
#[derive(Debug, Clone)]
pub struct Attribute<'a> {
    /// The name of the attribute.
    name: &'a str,
    /// The value of the attribute.
    value: &'a str,

    namespace: Option<crate::builder::Namespace<'a>>,
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
    /// use xml::builder::Attribute;
    /// let attribute = Attribute::new("name", "value");
    /// ```
    pub fn new(name: &'a str, value: &'a str) -> Self {
        Attribute {
            name,
            value,
            namespace: None,
        }
    }

    pub fn set_namespace(mut self, namespace: crate::builder::Namespace<'a>) -> Self {
        self.namespace = Some(namespace);
        self
    }

    pub fn get_namespaces(
        &self,
        namespaces_set: &mut std::collections::HashSet<crate::builder::Namespace<'a>>,
    ) {
        if let Some(namespace) = &self.namespace {
            namespaces_set.insert(namespace.clone());
        }
    }
}

impl crate::builder::NamespaceFmt for Attribute<'_> {
    /// Formats the attribute as a string in the format `name="value"`.
    fn ns_fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        alias_map: &HashMap<crate::builder::Namespace<'_>, &str>,
    ) -> std::fmt::Result {
        let namespace_alias = self
            .namespace
            .as_ref()
            .and_then(|ns| alias_map.get(ns))
            .copied();

        let name = if let Some(alias) = namespace_alias {
            format!("{}:{}", alias, self.name)
        } else {
            self.name.to_string()
        };

        write!(f, " {}=\"{}\"", name, self.value)?; // This line duplicates the name!
        Ok(())
    }
}
