use std::{borrow::Cow, collections::HashMap};

use crate::builder::{Attribute, Namespace};

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
    namespaces: Option<HashMap<Namespace<'a>, &'a str>>,
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
            namespaces: None,
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
    ///     .set_namespace(Namespace::new("name", "http://example.com"));
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
    pub fn add_namespace_alias(mut self, namespace: &'a str, alias: &'a str) -> Self {
        if self.namespaces.is_none() {
            self.namespaces = Some(HashMap::new());
        }

        let namespace = Namespace::new(namespace);

        self.namespaces
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
    pub fn set_text(mut self, text: impl Into<&'a str>) -> Self {
        self.content = Content::Text(std::borrow::Cow::Borrowed(text.into()));
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
}

impl crate::builder::NamespaceFmt for Element<'_> {
    /// Formats the element and its content as an XML string.
    fn ns_fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        parent_namespaces_map: Option<&HashMap<Namespace<'_>, &str>>,
    ) -> std::fmt::Result {
        let namespace_alias_map = match (parent_namespaces_map, &self.namespaces) {
            (None, None) => None,
            (None, Some(my_map)) => Some(Cow::Borrowed(my_map)),
            (Some(parent_map), None) => Some(Cow::Borrowed(parent_map)),
            (Some(parent_map), Some(my_map)) => Some({
                let mut merged_namespace = HashMap::new();

                merged_namespace.extend(parent_map.iter().map(|(ns, alias)| (ns.clone(), *alias)));
                merged_namespace.extend(my_map.iter().map(|(ns, alias)| (ns.clone(), *alias)));

                Cow::Owned(merged_namespace)
            }),
        };

        let alias = if let Some(namespace) = &self.namespace {
            if let Some(ref namespaces_map) = namespace_alias_map {
                namespaces_map.get(namespace).copied()
            } else {
                return Err(std::fmt::Error);
            }
        } else {
            None
        };

        let name = if let Some(alias) = alias {
            format!("{}:{}", alias, self.name)
        } else {
            self.name.to_string()
        };

        write!(f, "<{name}")?;

        if let Some(this_namespaces) = &self.namespaces {
            for (url, alias) in this_namespaces {
                write!(f, " xmlns:{alias}=\"{url}\"")?;
            }
        }

        for attribute in &self.attributes {
            attribute.ns_fmt(f, namespace_alias_map.as_ref().map(|v| &**v))?;
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
                    child.ns_fmt(f, namespace_alias_map.as_ref().map(|v| &**v))?;
                }
                write!(f, "</{name}>")?;
            }
        }
        Ok(())
    }
}
