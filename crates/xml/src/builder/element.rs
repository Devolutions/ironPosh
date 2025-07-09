use std::collections::HashMap;

use crate::builder::{Attribute, Namespace, NamespaceFmt};

#[derive(Debug, Clone)]
pub enum Content<'a> {
    /// Represents a text content within an XML element.
    Text(&'a str),
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
    pub fn set_namespace(mut self, namespace: Namespace<'a>) -> Self {
        self.namespace = Some(namespace);
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
    pub fn set_text(mut self, text: &'a str) -> Self {
        self.content = Content::Text(text);
        self
    }

    pub fn with_text(&mut self, text: &'a str) -> &mut Self {
        self.content = Content::Text(text);
        self
    }
}

impl crate::builder::NamespaceFmt for Element<'_> {
    /// Formats the element and its content as an XML string.
    fn ns_fmt(
        &self,
        f: &mut std::fmt::Formatter<'_>,
        namespaces_map: &HashMap<Namespace<'_>, &str>,
    ) -> std::fmt::Result {
        let alias = self
            .namespace
            .as_ref()
            .and_then(|ns| namespaces_map.get(ns));

        let name = if let Some(alias) = alias {
            format!("{}:{}", alias, self.name)
        } else {
            self.name.to_string()
        };

        write!(f, "<{name}")?;

        for attribute in &self.attributes {
            attribute.ns_fmt(f, namespaces_map)?;
        }

        match &self.content {
            Content::None => {
                write!(f, "/>")?;
            }
            Content::Text(value) => {
                write!(f, ">{value}</{name}>")?;
            }
            Content::Elements(children) => {
                writeln!(f, ">")?;
                for child in children {
                    // Write indented XML using recursive call
                    write!(f, "    ")?; // indent
                    child.ns_fmt(f, namespaces_map)?; // recursive call
                    writeln!(f)?; // newline
                }
                write!(f, "</{name}>")?;
            }
        }
        Ok(())
    }
}

pub struct RootElement<'a> {
    element: Element<'a>,
    namespaces: HashMap<Namespace<'a>, &'a str>,
}

impl<'a> RootElement<'a> {
    /// Creates a new instance of `RootElement` with the given element.
    ///
    /// # Arguments
    ///
    /// * `element` - The root element of the XML document.
    ///
    /// # Example
    ///
    /// ```
    /// use xml::builder::{Element, RootElement};
    /// let element = Element::new("root");
    /// let root_element = RootElement::new(element);
    /// ```
    pub fn new(element: Element<'a>) -> Self {
        RootElement {
            element,
            namespaces: HashMap::new(),
        }
    }

    pub fn set_alias(mut self, namespace: &'a str, alias: &'a str) -> Self {
        self.namespaces.insert(Namespace::new(namespace), alias);
        self
    }
}

impl std::fmt::Display for RootElement<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let alias = self
            .element
            .namespace
            .as_ref()
            .and_then(|ns| self.namespaces.get(ns));

        // Assemble the name with namespace if it exists
        // For example, <s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/">
        let name = if let Some(alias) = &alias {
            alias.to_string() + ":" + self.element.name
        } else {
            self.element.name.to_string()
        };
        write!(f, "<{name}")?;

        for (url, alias) in &self.namespaces {
            write!(f, " xmlns:{alias}=\"{url}\"")?;
        }

        for attribute in &self.element.attributes {
            attribute.ns_fmt(f, &self.namespaces)?;
        }

        match &self.element.content {
            Content::None => {
                write!(f, "/>")?;
            }
            Content::Text(value) => {
                write!(f, ">{value}</{name}>")?;
            }
            Content::Elements(children) => {
                writeln!(f, ">")?;
                for child in children {
                    // Write indented XML using recursive call
                    write!(f, "    ")?; // indent
                    child.ns_fmt(f, &self.namespaces)?; // recursive call
                    writeln!(f)?; // newline
                }
                write!(f, "</{name}>")?;
            }
        }
        Ok(())
    }
}
