use std::collections::HashSet;

use crate::{Attribute, Namespace};

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
    /// use xml_builder::Element;
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
    /// use xml_builder::{Element, Namespace};
    /// let element = Element::new("root")
    ///     .set_namespace(Namespace::new("name", "http://example.com"));
    /// ```
    pub fn set_namespace(mut self, namespace: Namespace<'a>) -> Self {
        self.namespace = Some(namespace);
        self
    }

    pub(crate) fn get_namespaces(&self, namespaces_set: &mut HashSet<Namespace<'a>>) {
        if let Some(namespace) = &self.namespace {
            if !namespaces_set.contains(namespace) {
                namespaces_set.insert(namespace.to_owned());
            }
        }

        if let Content::Elements(children) = &self.content {
            for child in children {
                child.get_namespaces(namespaces_set);
            }
        }

        for attribute in &self.attributes {
            attribute.get_namespaces(namespaces_set);
        }
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
    /// use xml_builder::{Element, Attribute};
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
    /// use xml_builder::Element;
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
    /// use xml_builder::Element;   
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

impl std::fmt::Display for Element<'_> {
    /// Formats the element and its content as an XML string.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let name = if let Some(namespace) = &self.namespace {
            namespace.alias.to_string() + ":" + self.name
        } else {
            self.name.to_string()
        };
        write!(f, "<{name}")?;

        for attribute in &self.attributes {
            write!(f, " {attribute}")?;
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
                    let child_string = child.to_string();
                    for line in child_string.lines() {
                        writeln!(f, "    {line}")?;
                    }
                }
                write!(f, "</{name}>")?;
            }
        }
        Ok(())
    }
}

pub struct RootElement<'a> {
    element: Element<'a>,
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
    /// use xml_builder::{Element, RootElement};
    /// let element = Element::new("root");
    /// let root_element = RootElement::new(element);
    /// ```
    pub fn new(element: Element<'a>) -> Self {
        RootElement { element }
    }
}

impl std::fmt::Display for RootElement<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut namespace_set = HashSet::new();
        self.element.get_namespaces(&mut namespace_set);

        // Assemble the name with namespace if it exists
        // For example, <s:Envelope xmlns:s="http://schemas.xmlsoap.org/soap/envelope/">
        let name = if let Some(namespace) = &self.element.namespace {
            namespace.alias.to_string() + ":" + self.element.name
        } else {
            self.element.name.to_string()
        };
        write!(f, "<{name}")?;

        for namespace in &namespace_set {
            write!(f, " xmlns:{}=\"{}\"", namespace.alias, namespace.url)?;
        }

        for attribute in &self.element.attributes {
            write!(f, " {attribute}")?;
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
                    let child_string = child.to_string();
                    for line in child_string.lines() {
                        writeln!(f, "    {line}")?;
                    }
                }
                write!(f, "</{name}>")?;
            }
        }
        Ok(())
    }
}
