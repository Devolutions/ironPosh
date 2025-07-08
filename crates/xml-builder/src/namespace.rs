use std::hash::Hash;

/// Represents a namespace in XML.
#[derive(Debug, Clone, Eq)]
pub struct Namespace<'a> {
    pub url: &'a str,
    pub alias: &'a str,
}

impl PartialEq for Namespace<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.url == other.url
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
    /// use xml_builder::Namespace;
    /// let namespace = Namespace::new("xmlns", "http://example.com");
    /// ```
    pub fn new(prefix: &'a str, uri: &'a str) -> Self {
        Namespace {
            url: uri,
            alias: prefix,
        }
    }
}
