use crate::{
    define_tagname,
    traits::{Tag, TagValue},
};

pub const PWSH_NS: &str = "http://schemas.microsoft.com/wbem/wsman/1/windows/shell";
pub const PWSH_NS_ALIAS: &str = "rsp";

// Define tag names for PowerShell remoting shell elements
define_tagname!(InputStreams, Some(PWSH_NS));
define_tagname!(OutputStreams, Some(PWSH_NS));
define_tagname!(CreationXml, Some(PWSH_NS));

#[derive(Debug, Clone)]
pub struct ShellElement;

// Custom implementation for ShellElement to have the correct tag name
impl crate::traits::TagName for ShellElement {
    fn tag_name(&self) -> &'static str {
        "Shell"
    }

    fn namespace(&self) -> Option<&'static str> {
        Some(PWSH_NS)
    }
}

#[derive(Debug, Clone)]
pub struct ShellContent<'a> {
    pub input_stream: &'a str,
    pub output_stream: &'a str,
    pub creation_xml: &'a str,
}

impl<'a> TagValue<'a> for ShellContent<'a> {
    fn into_element(
        self,
        name: &'static str,
        namespace: Option<&'static str>,
    ) -> xml::builder::Element<'a> {
        let mut element = xml::builder::Element::new(name);

        if let Some(ns) = namespace {
            element = element.set_namespace(ns);
        }

        let input_streams = self
            .input_stream
            .into_element("InputStreams", Some(PWSH_NS));
        let output_streams = self
            .output_stream
            .into_element("OutputStreams", Some(PWSH_NS));
        let creation_xml = self.creation_xml.into_element("CreationXml", Some(PWSH_NS));

        element
            .add_namespace_alias(PWSH_NS, PWSH_NS_ALIAS)
            .add_child(input_streams)
            .add_child(output_streams)
            .add_child(creation_xml)
    }
}

#[derive(Debug, Clone, typed_builder::TypedBuilder)]
pub struct Shell<'a> {
    #[builder(setter(into))]
    pub input_stream: &'a str,
    #[builder(setter(into))]
    pub output_stream: &'a str,
    #[builder(setter(into))]
    pub creation_xml: &'a str,
}

impl<'a> Shell<'a> {
    pub fn into_tag(self) -> Tag<'a, ShellContent<'a>, ShellElement> {
        let content = ShellContent {
            input_stream: self.input_stream,
            output_stream: self.output_stream,
            creation_xml: self.creation_xml,
        };

        Tag::new(ShellElement, content)
    }

    pub fn into_element(self) -> xml::builder::Element<'a> {
        self.into_tag().into_element()
    }
}

pub fn shell_builder<'a>() -> ShellBuilder<'a> {
    Shell::builder()
}
