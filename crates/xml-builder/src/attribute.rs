/// Represents an XML attribute with a name and value.
pub struct Attribute<'a> {
    /// The name of the attribute.
    name: &'a str,
    /// The value of the attribute.
    value: &'a str,
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
        }
    }
}

impl std::fmt::Display for Attribute<'_> {
    /// Formats the attribute as a string in the format `name="value"`.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}=\"{}\"", self.name, self.value)?;
        Ok(())
    }
}