use std::{borrow::Cow, collections::HashMap};

use tracing::error;

use crate::builder::{AliasMap, Attribute, Namespace, NamespaceWrite, XmlBuilderError};

#[derive(Debug, Clone)]
pub enum Content<'a> {
    /// Represents a text content within an XML element.
    Text(Cow<'a, str>),
    /// Represents a child element within an XML element.
    Elements(Vec<Element<'a>>),

    None,
}

/// Represents an XML element.
#[derive(Debug, Clone)]
pub struct Element<'a> {
    /// The name of the element.
    name: &'a str,
    /// The namespaces associated with the element.
    namespace: Option<Namespace<'a>>,
    /// The attributes of the element.
    attributes: Vec<Attribute<'a>>,
    /// The child elements of the element.
    content: Content<'a>,
    /// The namespaces declaretions for this and child elements.
    namespaces_declaration: Option<HashMap<Namespace<'a>, Option<&'a str>>>,
}

impl<'a> Element<'a> {
    /// Creates a new instance of `Element` with the given name.
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the element.
    ///
    /// # Example
    ///
    /// ```
    /// use xml::builder::Element;
    /// let element = Element::new("root");
    /// ```
    pub fn new(name: &'a str) -> Self {
        Element {
            name,
            namespace: None,
            attributes: Vec::new(),
            content: Content::None,
            namespaces_declaration: None,
        }
    }

    /// Adds a namespace to the element and returns a modified `Element`.
    ///
    /// # Arguments
    ///
    /// * `namespace` - The namespace to be added.
    ///
    /// # Example
    ///
    /// ```
    /// use xml::builder::{Element, Namespace};
    /// let element = Element::new("root")
    ///     .set_namespace(Namespace::new("http://example.com"));
    /// ```
    pub fn set_namespace(mut self, ns: impl Into<Namespace<'a>>) -> Self {
        self.namespace = Some(ns.into());
        self
    }

    pub fn set_namespace_optional(mut self, ns: Option<impl Into<Namespace<'a>>>) -> Self {
        if let Some(ns) = ns {
            self.namespace = Some(ns.into());
        } else {
            self.namespace = None;
        }
        self
    }

    /// Namespace alias map is used to resolve namespace prefixes.
    /// for example, if the element has a namespace with a prefix "ns",
    /// the alias map will contain an entry for "ns" pointing to the namespace URI.
    ///  <SomeElement xmlns:ns="http://example.com/ns">
    ///     <ns:SomeChildElement/>
    ///  </SomeElement>
    ///
    pub fn add_namespace_declaration(mut self, namespace: &'a str, alias: Option<&'a str>) -> Self {
        if self.namespaces_declaration.is_none() {
            self.namespaces_declaration = Some(HashMap::new());
        }

        let namespace = Namespace::new(namespace);

        self.namespaces_declaration
            .as_mut()
            .expect("Namespaces should be initialized")
            .insert(namespace.clone(), alias);

        self
    }

    /// Adds an attribute to the element and returns a modified `Element`.
    ///
    /// # Arguments
    ///
    /// * `attribute` - The attribute to be added.
    ///
    /// # Example
    ///
    /// ```
    /// use xml::builder::{Element, Attribute};
    /// let element = Element::new("root")
    ///     .add_attribute(Attribute::new("attr1", "value1"));
    /// ```
    pub fn add_attribute(mut self, attribute: Attribute<'a>) -> Self {
        self.attributes.push(attribute);
        self
    }

    /// Adds a child element to the element and returns a modified `Element`.
    ///
    /// # Arguments
    ///
    /// * `child` - The child element to be added.
    ///
    /// # Example
    ///
    /// ```
    /// use xml::builder::Element;
    /// let child = Element::new("child");
    /// let element = Element::new("root")
    ///     .add_child(child);
    /// ```
    pub fn add_child(mut self, child: Element<'a>) -> Self {
        match self.content {
            Content::None | Content::Text(_) => {
                self.content = Content::Elements(vec![child]);
            }
            Content::Elements(ref mut children) => {
                children.push(child);
            }
        }
        self
    }

    pub fn add_children(mut self, children: Vec<Element<'a>>) -> Self {
        for child in children {
            self = self.add_child(child);
        }
        self
    }

    /// Sets the text content of the element and returns a modified `Element`.
    ///
    /// # Arguments
    ///
    /// * `text` - The text content to be set.
    ///
    /// # Example
    ///
    /// ```
    /// use xml::builder::Element;   
    /// let element = Element::new("root")
    ///    .set_text("This is some text content.");
    ///     
    /// ```
    pub fn set_text(mut self, text: impl Into<Cow<'a, str>>) -> Self {
        self.content = Content::Text(text.into());
        self
    }

    pub fn set_text_owned(mut self, text: String) -> Self {
        self.content = Content::Text(std::borrow::Cow::Owned(text));
        self
    }

    pub fn with_text(&mut self, text: &'a str) -> &mut Self {
        self.content = Content::Text(std::borrow::Cow::Borrowed(text));
        self
    }

    pub fn with_text_owned(&mut self, text: String) -> &mut Self {
        self.content = Content::Text(std::borrow::Cow::Owned(text));
        self
    }

    pub fn to_xml_string(&self) -> Result<String, crate::XmlError> {
        let mut buf = Vec::new();
        self.ns_write(&mut buf, None)?;
        Ok(String::from_utf8(buf).map_err(XmlBuilderError::from)?)
    }
}

#[derive(Debug, Clone)]
pub enum AliasStatus {
    ElementHasNoNamespace,
    NamespaceFoundWithAlias(String),
    NamespaceFoundWithoutAlias,
    NamespaceNotFoundInDeclaration,
    NamespaceDeclarationMapMissing,
}

impl<'a> crate::builder::NamespaceWrite<'a> for Element<'a> {
    fn ns_write<W: std::io::Write>(
        &self,
        w: &mut W,
        parent_decl_map: Option<&AliasMap<'a>>,
    ) -> Result<(), XmlBuilderError> {
        // Merge alias maps (child overrides parent) – same logic as before:
        let decl_map = match (parent_decl_map, &self.namespaces_declaration) {
            (None, None) => None,
            (None, Some(m)) => Some(std::borrow::Cow::Borrowed(m)),
            (Some(p), None) => Some(std::borrow::Cow::Borrowed(p)),
            (Some(p), Some(m)) => {
                let mut merged = std::collections::HashMap::new();
                merged.extend(p.iter().map(|(ns, a)| (ns.clone(), *a)));
                merged.extend(m.iter().map(|(ns, a)| (ns.clone(), *a)));
                Some(std::borrow::Cow::Owned(merged))
            }
        };

        // Resolve the element name with namespace/alias
        let name = match (&self.namespace, &decl_map) {
            (None, _) => self.name.to_string(),
            (Some(ns), None) => {
                return Err(XmlBuilderError::MissingAliasMapForElement {
                    tag: self.name.to_string(),
                    ns: ns.url.to_string(),
                });
            }
            (Some(ns), Some(map)) => match map.get(ns) {
                Some(Some(alias)) => format!("{alias}:{}", self.name),
                Some(None) => {
                    return Err(XmlBuilderError::NamespaceHasNoAlias {
                        tag: self.name.to_string(),
                        ns: ns.url.to_string(),
                    })
                }
                None => {
                    return Err(XmlBuilderError::NamespaceNotDeclared {
                        tag: self.name.to_string(),
                        ns: ns.url.to_string(),
                    })
                }
            },
        };

        // Write start tag + namespace declarations (unchanged behavior)
        w.write_fmt(format_args!("<{name}"))?;
        if let Some(this_ns) = &self.namespaces_declaration {
            for (url, alias) in this_ns {
                if let Some(alias) = alias {
                    w.write_fmt(format_args!(" xmlns:{alias}=\"{url}\""))?;
                } else {
                    w.write_fmt(format_args!(" xmlns=\"{url}\""))?;
                }
            }
        }

        // Attributes
        for a in &self.attributes {
            a.ns_write(w, decl_map.as_deref())?;
        }

        // Content
        match &self.content {
            Content::None => {
                w.write_all(b"/>")?;
            }
            Content::Text(t) => {
                w.write_fmt(format_args!(">{t}</{name}>"))?;
            }
            Content::Elements(children) => {
                w.write_all(b">")?;
                for c in children {
                    c.ns_write(w, decl_map.as_deref())?;
                }
                w.write_fmt(format_args!("</{name}>"))?;
            }
        }
        Ok(())
    }
}

impl crate::builder::NamespaceFmt for Element<'_> {
    /// Formats the element and its content as an XML string.
    fn ns_fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        parent_declaration_map: Option<&HashMap<Namespace<'_>, Option<&str>>>,
    ) -> std::fmt::Result {
        let namespace_declaration_map = match (parent_declaration_map, &self.namespaces_declaration)
        {
            // The case where no declarations are present, and the current element has no namespace declarations.
            (None, None) => None,
            // The case where no declarations are present in parent, this should only happen at the root element.
            (None, Some(my_map)) => Some(Cow::Borrowed(my_map)),
            // The case where parent declarations are present, and the current element has no namespace declarations.
            (Some(parent_map), None) => Some(Cow::Borrowed(parent_map)),
            // The case where both parent and current element have namespace declarations.
            // We merge the two maps, giving priority to the current element's declarations.
            (Some(parent_map), Some(my_map)) => Some({
                let mut merged_namespace = HashMap::new();

                merged_namespace.extend(parent_map.iter().map(|(ns, alias)| (ns.clone(), *alias)));
                merged_namespace.extend(my_map.iter().map(|(ns, alias)| (ns.clone(), *alias)));

                Cow::Owned(merged_namespace)
            }),
        };

        let alias = 'alias: {
            if let Some(namespace) = &self.namespace {
                let Some(ref namespaces_map) = namespace_declaration_map else {
                    break 'alias AliasStatus::NamespaceDeclarationMapMissing;
                };

                match namespaces_map.get(namespace) {
                    Some(Some(alias)) => AliasStatus::NamespaceFoundWithAlias(alias.to_string()),
                    /*
                    For cases where the namespace is found but no alias is provided. right now this is only used for
                       <creationXml
                           xmlns="http://schemas.microsoft.com/powershell/Microsoft.PowerShell">
                       > ....

                    Notice that it declares a namespace without an alias
                    */
                    Some(None) => AliasStatus::NamespaceFoundWithoutAlias,
                    None => AliasStatus::NamespaceNotFoundInDeclaration,
                }
            } else {
                AliasStatus::ElementHasNoNamespace
            }
        };

        let name = match alias {
            AliasStatus::ElementHasNoNamespace => self.name.to_string(),
            AliasStatus::NamespaceFoundWithAlias(alias) => {
                format!("{}:{}", alias, self.name)
            }
            AliasStatus::NamespaceFoundWithoutAlias => {
                error!(
                    target: "xml_namespace",
                    alias_status = ?alias,
                    tag_name = self.name,
                    "element has no alias but namespace is present"
                );
                return Err(std::fmt::Error);
            }
            AliasStatus::NamespaceNotFoundInDeclaration => {
                error!(
                    target: "xml_namespace",
                    alias_status = ?alias,
                    tag_name = self.name,
                    expected_namespace = ?self.namespace,
                    namespace_declaration_map = ?namespace_declaration_map,
                    self_namespaces_declaration = ?self.namespaces_declaration,
                    "namespace not found in declaration map for element"
                );
                return Err(std::fmt::Error);
            }
            AliasStatus::NamespaceDeclarationMapMissing => {
                error!(
                    target: "xml_namespace",
                    alias_status = ?alias,
                    tag_name = self.name,
                    missing_namespace = ?self.namespace,
                    namespace_declaration_map = ?namespace_declaration_map,
                    "namespace alias not found for element"
                );
                return Err(std::fmt::Error);
            }
        };

        write!(f, "<{name}")?;

        if let Some(this_namespaces) = &self.namespaces_declaration {
            for (url, alias) in this_namespaces {
                if let Some(alias) = alias {
                    write!(f, " xmlns:{alias}=\"{url}\"")?;
                } else {
                    write!(f, " xmlns=\"{url}\"")?;
                }
            }
        }

        for attribute in &self.attributes {
            attribute.ns_fmt(f, namespace_declaration_map.as_deref())?;
        }

        match &self.content {
            Content::None => {
                write!(f, "/>")?;
            }
            Content::Text(value) => {
                write!(f, ">{value}</{name}>")?;
            }
            Content::Elements(children) => {
                write!(f, ">")?;
                for child in children {
                    child.ns_fmt(f, namespace_declaration_map.as_deref())?;
                }
                write!(f, "</{name}>")?;
            }
        }
        Ok(())
    }
}
