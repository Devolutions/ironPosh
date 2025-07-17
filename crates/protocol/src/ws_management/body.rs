use xml::builder::Element;

use crate::cores::{Tag, TagValue, tag_name::*, tag_value::Text};

pub fn body_builder<'a>() -> WsManagementBodyBuilder<'a> {
    WsManagementBody::builder()
}

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
    fn into_element(self, name: &'static str, namespace: Option<&'static str>) -> Element<'a> {
        let element = Element::new(name).set_namespace_optional(namespace);

        eprintln!("EnumerateValue into_element: {element:?}, not finshied");
        // if let Some(optimize) = self.optimize_enumeration {
        //     let opt_elem = Element::new("OptimizeEnumeration")
        //         .set_namespace(wsman_ns!())
        //         .set_text(if optimize { "true" } else { "false" });
        //     element = element.add_child(opt_elem);
        // }

        // if let Some(max) = self.max_elements {
        //     let max_elem = Element::new("MaxElements")
        //         .set_namespace(wsman_ns!())
        //         .set_text_owned(max.to_string());
        //     element = element.add_child(max_elem);
        // }

        // if let Some(filter) = self.filter {
        //     let filter_elem = Element::new("Filter")
        //         .set_namespace(WSMAN_NAMESPACE)
        //         .set_text(filter);

        //     element = element.add_child(filter_elem);
        // }

        element
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
    fn into_element(self, name: &'static str, namespace: Option<&'static str>) -> Element<'a> {
        let mut element = Element::new(name).set_namespace_optional(
            namespace.or(Some("http://schemas.xmlsoap.org/ws/2004/09/enumeration")),
        );

        let context_elem = Element::new("EnumerationContext")
            .set_namespace("http://schemas.xmlsoap.org/ws/2004/09/enumeration")
            .set_text(self.enumeration_context);

        element = element.add_child(context_elem);

        if let Some(max) = self.max_elements {
            let max_elem = Element::new("MaxElements")
                // .set_namespace(wsman_ns!())
                .set_text_owned(max.to_string());

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
    fn into_element(self, name: &'static str, namespace: Option<&'static str>) -> Element<'a> {
        let context_elem = Element::new("EnumerationContext")
            .set_namespace("http://schemas.xmlsoap.org/ws/2004/09/enumeration")
            .set_text(self.enumeration_context);

        Element::new(name)
            .set_namespace_optional(
                namespace.or(Some("http://schemas.xmlsoap.org/ws/2004/09/enumeration")),
            )
            .add_child(context_elem)
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
    fn into_element(self, name: &'static str, namespace: Option<&'static str>) -> Element<'a> {
        let context_elem = Element::new("EnumerationContext")
            .set_namespace("http://schemas.xmlsoap.org/ws/2004/09/enumeration")
            .set_text(self.enumeration_context);

        Element::new(name)
            .set_namespace_optional(
                namespace.or(Some("http://schemas.xmlsoap.org/ws/2004/09/enumeration")),
            )
            .add_child(context_elem)
    }
}

// Main body builder structure
#[derive(typed_builder::TypedBuilder, Debug, Clone)]
pub struct WsManagementBody<'a> {
    #[builder(default, setter(strip_option, into))]
    pub identify: Option<Tag<'a, (), Identify>>,
    #[builder(default, setter(strip_option, into))]
    pub get: Option<Tag<'a, Text<'a>, Get>>,
    #[builder(default, setter(strip_option, into))]
    pub put: Option<Tag<'a, Text<'a>, Put>>,
    #[builder(default, setter(strip_option, into))]
    pub create: Option<Tag<'a, Text<'a>, Create>>,
    #[builder(default, setter(strip_option, into))]
    pub delete: Option<Tag<'a, Text<'a>, Delete>>,
    #[builder(default, setter(strip_option, into))]
    pub rename: Option<Tag<'a, Text<'a>, Delete>>,
    #[builder(default, setter(strip_option, into))]
    pub enumerate: Option<Tag<'a, EnumerateValue<'a>, Enumerate>>,
    #[builder(default, setter(strip_option, into))]
    pub pull: Option<Tag<'a, PullValue<'a>, Pull>>,
    #[builder(default, setter(strip_option, into))]
    pub release: Option<Tag<'a, ReleaseValue<'a>, Release>>,
    #[builder(default, setter(strip_option, into))]
    pub get_status: Option<Tag<'a, GetStatusValue<'a>, GetStatus>>,
}
