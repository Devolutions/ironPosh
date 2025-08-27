/// Represents an XML declaration.
pub struct Declaration<'a> {
    /// The XML version.
    version: &'a str,
    /// The encoding used for the XML document.
    encoding: &'a str,
    /// The standalone status of the XML document (optional).
    standalone: Option<bool>,
}

impl<'a> Declaration<'a> {
    /// Creates a new instance of `Declaration` with the given version and encoding.
    ///
    /// # Arguments
    ///
    /// * `version` - The XML version.
    /// * `encoding` - The encoding used for the XML document.
    ///
    /// # Example
    ///
    /// ```
    /// use xml::builder::Declaration;
    /// let declaration = Declaration::new("1.0", "UTF-8");
    /// ```
    pub fn new(version: &'a str, encoding: &'a str) -> Self {
        Declaration {
            version,
            encoding,
            standalone: None,
        }
    }

    /// Sets the standalone status of the XML document and returns a modified `Declaration`.
    ///
    /// # Arguments
    ///
    /// * `standalone` - The standalone status of the XML document.
    ///
    /// # Example
    ///
    /// ```
    /// use xml::builder::Declaration;
    /// let declaration = Declaration::new("1.0", "UTF-8")
    ///     .with_standalone(true);
    /// ```
    pub fn with_standalone(mut self, standalone: bool) -> Self {
        self.standalone = Some(standalone);
        self
    }

    pub fn write<W: std::io::Write>(&self, w: &mut W) -> std::io::Result<()> {
        w.write_fmt(format_args!(
            "<?xml version=\"{}\" encoding=\"{}\"",
            self.version, self.encoding
        ))?;
        if let Some(standalone) = self.standalone {
            let s = if standalone { "yes" } else { "no" };
            w.write_fmt(format_args!(" standalone=\"{s}\""))?;
        }
        w.write_all(b"?>")?;
        Ok(())
    }
}

impl<'a> std::fmt::Display for Declaration<'a> {
    /// Formats the declaration as an XML declaration string.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            r#"<?xml version="{}" encoding="{}""#,
            self.version, self.encoding
        )?;

        if let Some(standalone) = self.standalone {
            let standalone_as_string = if standalone { "yes" } else { "no" };

            write!(f, r#" standalone="{standalone_as_string}""#)?;
        }

        write!(f, "?>")?;
        Ok(())
    }
}
