use xml::builder::Element;

use crate::cores::{TagValue, tag_value::Text};

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
    fn into_element(self, element: Element<'a>) -> Element<'a> {
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
    fn into_element(self, mut element: Element<'a>) -> Element<'a> {
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
    fn into_element(self, element: Element<'a>) -> Element<'a> {
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
    fn into_element(self, element: Element<'a>) -> Element<'a> {
        let context_elem = Element::new("EnumerationContext")
            .set_namespace("http://schemas.xmlsoap.org/ws/2004/09/enumeration")
            .set_text(self.enumeration_context);

        element.add_child(context_elem)
    }
}
