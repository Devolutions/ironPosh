use protocol_macros::{SimpleTagValue, SimpleXmlDeserialize};
use xml::{
    builder::Element,
    parser::{XmlDeserialize, XmlVisitor},
};

use crate::{
    cores::{ResourceURI, SelectorSet, Tag, TagValue, tag_name::*, tag_value::Text},
    ws_management::SelectorSetValue,
};

// Enumeration operations
#[derive(Debug, Clone)]
pub struct EnumerateValue<'a> {
    pub optimize_enumeration: Option<bool>,
    pub max_elements: Option<u32>,
    pub filter: Option<Text<'a>>,
}

impl<'a> EnumerateValue<'a> {
    pub fn new() -> Self {
        Self {
            optimize_enumeration: None,
            max_elements: None,
            filter: None,
        }
    }

    pub fn with_optimization(mut self, optimize: bool) -> Self {
        self.optimize_enumeration = Some(optimize);
        self
    }

    pub fn with_max_elements(mut self, max: u32) -> Self {
        self.max_elements = Some(max);
        self
    }

    pub fn with_filter(mut self, filter: Text<'a>) -> Self {
        self.filter = Some(filter);
        self
    }
}

impl<'a> Default for EnumerateValue<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> TagValue<'a> for EnumerateValue<'a> {
    fn append_to_element(self, _element: Element<'a>) -> Element<'a> {
        todo!("[EnumerateValue] Implement into_element");
    }
}

#[derive(Debug, Clone)]
pub struct PullValue<'a> {
    pub enumeration_context: Text<'a>,
    pub max_elements: Option<u32>,
}

impl<'a> PullValue<'a> {
    pub fn new(enumeration_context: Text<'a>) -> Self {
        Self {
            enumeration_context,
            max_elements: None,
        }
    }

    pub fn with_max_elements(mut self, max: u32) -> Self {
        self.max_elements = Some(max);
        self
    }
}

impl<'a> TagValue<'a> for PullValue<'a> {
    fn append_to_element(self, mut element: Element<'a>) -> Element<'a> {
        let context_elem = Element::new("EnumerationContext")
            .set_namespace("http://schemas.xmlsoap.org/ws/2004/09/enumeration")
            .set_text(self.enumeration_context);

        element = element.add_child(context_elem);

        if let Some(max) = self.max_elements {
            let max_elem = Element::new("MaxElements").set_text_owned(max.to_string());

            element = element.add_child(max_elem);
        }

        element
    }
}

#[derive(Debug, Clone)]
pub struct ReleaseValue<'a> {
    pub enumeration_context: Text<'a>,
}

impl<'a> ReleaseValue<'a> {
    pub fn new(enumeration_context: Text<'a>) -> Self {
        Self {
            enumeration_context,
        }
    }
}

impl<'a> TagValue<'a> for ReleaseValue<'a> {
    fn append_to_element(self, element: Element<'a>) -> Element<'a> {
        let context_elem = Element::new("EnumerationContext")
            .set_namespace("http://schemas.xmlsoap.org/ws/2004/09/enumeration")
            .set_text(self.enumeration_context);

        element.add_child(context_elem)
    }
}

#[derive(Debug, Clone)]
pub struct GetStatusValue<'a> {
    pub enumeration_context: Text<'a>,
}

impl<'a> GetStatusValue<'a> {
    pub fn new(enumeration_context: Text<'a>) -> Self {
        Self {
            enumeration_context,
        }
    }
}

impl<'a> TagValue<'a> for GetStatusValue<'a> {
    fn append_to_element(self, element: Element<'a>) -> Element<'a> {
        let context_elem = Element::new("EnumerationContext")
            .set_namespace("http://schemas.xmlsoap.org/ws/2004/09/enumeration")
            .set_text(self.enumeration_context);

        element.add_child(context_elem)
    }
}

#[derive(Debug, Clone, SimpleTagValue, SimpleXmlDeserialize)]
pub struct ReferenceParametersValue<'a> {
    pub resource_uri: Tag<'a, Text<'a>, ResourceURI>,
    pub selector_set: Tag<'a, SelectorSetValue, SelectorSet>,
}

#[derive(Debug, Clone, SimpleTagValue, SimpleXmlDeserialize)]
pub struct ResourceCreatedValue<'a> {
    pub address: Tag<'a, Text<'a>, Address>,
    pub reference_parameters: Tag<'a, ReferenceParametersValue<'a>, ReferenceParameters>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use xml::parser::XmlDeserialize;

    #[test]
    fn test_resource_created_value_deserialize() {
        let xml = r#"
            <x:ResourceCreated 
                xmlns:x="http://schemas.xmlsoap.org/ws/2004/09/transfer"
                xmlns:a="http://schemas.xmlsoap.org/ws/2004/08/addressing"
                xmlns:w="http://schemas.dmtf.org/wbem/wsman/1/wsman.xsd"
            >
    <a:Address>
        http://10.10.0.3:5985/wsman?PSVersion=7.4.10
        </a:Address>
    <a:ReferenceParameters>
        <w:ResourceURI>
            http://schemas.microsoft.com/powershell/Microsoft.PowerShell
            </w:ResourceURI>
        <w:SelectorSet>
            <w:Selector
                Name="ShellId">
                2D6534D0-6B12-40E3-B773-CBA26459CFA8
                </w:Selector>
            </w:SelectorSet>
        </a:ReferenceParameters>
    </x:ResourceCreated>
        "#;

        let element = xml::parser::parse(xml).unwrap();
        let root = element.root_element();
        let tag: Tag<'_, ResourceCreatedValue, ResourceCreated> = Tag::from_node(root).unwrap();
        let value = tag.value;

        assert_eq!(
            value.address.value,
            "http://10.10.0.3:5985/wsman?PSVersion=7.4.10".into()
        );
        assert_eq!(
            value
                .reference_parameters
                .as_ref()
                .resource_uri
                .as_ref()
                .as_ref(),
            "http://schemas.microsoft.com/powershell/Microsoft.PowerShell"
        );
    }
}
