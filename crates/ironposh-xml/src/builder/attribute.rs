use std::{borrow::Cow, collections::HashMap};

/// Represents an XML attribute with a name and value.
#[derive(Debug, Clone)]
pub struct Attribute<'a> {
    /// The name of the attribute.
    name: &'a str,
    /// The value of the attribute.
    value: Cow<'a, str>,

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
    /// use ironposh_xml::builder::Attribute;
    /// let attribute = Attribute::new("name", "value");
    /// ```
    pub fn new(name: &'a str, value: impl Into<Cow<'a, str>>) -> Self {
        Attribute {
            name,
            value: value.into(),
            namespace: None,
        }
    }

    pub fn new_with_namespace(
        name: &'a str,
        value: impl Into<Cow<'a, str>>,
        namespace: Option<impl Into<crate::builder::Namespace<'a>>>,
    ) -> Self {
        Attribute {
            name,
            value: value.into(),
            namespace: namespace.map(Into::into),
        }
    }

    pub fn set_namespace(mut self, namespace: impl Into<crate::builder::Namespace<'a>>) -> Self {
        self.namespace = Some(namespace.into());
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

impl<'a> crate::builder::NamespaceWrite<'a> for Attribute<'a> {
    fn ns_write<W: std::io::Write>(
        &self,
        w: &mut W,
        alias_map: Option<&crate::builder::AliasMap<'a>>,
    ) -> Result<(), crate::builder::XmlBuilderError> {
        let ns_alias = if let Some(map) = alias_map {
            self.namespace.as_ref().and_then(|ns| map.get(ns)).copied()
        } else if let Some(ns) = &self.namespace {
            return Err(
                crate::builder::XmlBuilderError::MissingAliasMapForAttribute {
                    attr: self.name.to_string(),
                    ns: ns.url.to_string(),
                },
            );
        } else {
            None
        };

        let name = if let Some(Some(alias)) = ns_alias {
            format!("{alias}:{}", self.name)
        } else {
            self.name.to_string()
        };

        w.write_fmt(format_args!(" {}=\"{}\"", name, self.value))?;
        Ok(())
    }
}

impl crate::builder::NamespaceFmt for Attribute<'_> {
    /// Formats the attribute as a string in the format `name="value"`.
    fn ns_fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        alias_map: Option<&HashMap<super::namespace::Namespace<'_>, Option<&str>>>,
    ) -> std::fmt::Result {
        let namespace_alias = if let Some(alias_map) = alias_map {
            self.namespace
                .as_ref()
                .and_then(|ns| alias_map.get(ns))
                .copied()
        } else if let Some(ns) = &self.namespace {
            eprintln!("No namespace alias map provided for attribute: {ns}");
            return Err(std::fmt::Error);
        } else {
            None
        };

        let name = if let Some(Some(alias)) = namespace_alias {
            format!("{}:{}", alias, self.name)
        } else {
            self.name.to_string()
        };

        write!(f, " {}=\"{}\"", name, self.value)?;
        Ok(())
    }
}
