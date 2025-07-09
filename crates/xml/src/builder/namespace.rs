use core::fmt;
use std::hash::Hash;

/// Represents a namespace in XML.
#[derive(Debug, Clone, Eq)]
pub struct Namespace<'a> {
    pub url: &'a str,
}

impl PartialEq for Namespace<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.url == other.url
    }
}

impl fmt::Display for Namespace<'_> {
    /// Formats the namespace as a string in the format `xmlns:prefix="uri"`.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.url.fmt(f)
    }
}

impl Hash for Namespace<'_> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.url.hash(state);
    }
}

impl<'a> Namespace<'a> {
    /// Creates a new instance of `Namespace` with the given prefix and URI.
    ///
    /// # Arguments
    ///
    /// * `prefix` - The prefix for the namespace.
    /// * `uri` - The URI associated with the namespace.
    ///
    /// # Example
    ///
    /// ```
    /// use xml::builder::Namespace;
    /// let namespace = Namespace::new("xmlns", "http://example.com");
    /// ```
    pub fn new(uri: &'a str) -> Self {
        Namespace { url: uri }
    }
}
