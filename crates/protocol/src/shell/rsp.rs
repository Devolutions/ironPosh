use crate::soap::Value;

pub const PWSH_NS: &str = "http://schemas.microsoft.com/wbem/wsman/1/windows/shell";
pub const PWSH_NS_ALIAS: &str = "rsp";

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
    pub fn into_element(self) -> xml::builder::Element<'a> {
        let Shell {
            input_stream,
            output_stream,
            creation_xml,
        } = self;

        let input_stream = input_stream.into_element("InputStreams");
        let output_stream = output_stream.into_element("OutputStreams");
        let creation_xml = creation_xml.into_element("CreationXml");

        let children = vec![input_stream, output_stream, creation_xml]
            .into_iter()
            .map(|c| c.set_namespace(PWSH_NS))
            .collect::<Vec<_>>();

        xml::builder::Element::new("Shell")
            .set_namespace(PWSH_NS)
            .add_namespace_alias(PWSH_NS, PWSH_NS_ALIAS)
            .add_children(children)
    }
}
