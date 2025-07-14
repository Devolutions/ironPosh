use xml::builder::Element;

use crate::{
    define_tagname,
    traits::{Tag, TagValue},
    ws_management::WSMAN_NAMESPACE,
    wsman_ns,
};

pub fn body_builder<'a>() -> WsManagementBodyBuilder<'a> {
    WsManagementBody::builder()
}

define_tagname!(Identify, Some(WSMAN_NAMESPACE));
define_tagname!(Get, Some(WSMAN_NAMESPACE));
define_tagname!(Put, Some(WSMAN_NAMESPACE));
define_tagname!(Create, Some(WSMAN_NAMESPACE));
define_tagname!(Delete, Some(WSMAN_NAMESPACE));
define_tagname!(Enumerate, Some(WSMAN_NAMESPACE));
define_tagname!(Pull, Some(WSMAN_NAMESPACE));
define_tagname!(Release, Some(WSMAN_NAMESPACE));
define_tagname!(GetStatus, Some(WSMAN_NAMESPACE));

// Enumeration operations
#[derive(Debug, Clone)]
pub struct EnumerateValue<'a> {
    pub optimize_enumeration: Option<bool>,
    pub max_elements: Option<u32>,
    pub filter: Option<&'a str>,
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

    pub fn with_filter(mut self, filter: &'a str) -> Self {
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
        let mut element = Element::new(name).set_namespace_optional(namespace.or(Some(WSMAN_NAMESPACE)));

        if let Some(optimize) = self.optimize_enumeration {
            let opt_elem = Element::new("OptimizeEnumeration")
                .set_namespace(wsman_ns!())
                .set_text(if optimize { "true" } else { "false" });
            element = element.add_child(opt_elem);
        }

        if let Some(max) = self.max_elements {
            let max_elem = Element::new("MaxElements")
                .set_namespace(wsman_ns!())
                .set_text_owned(max.to_string());
            element = element.add_child(max_elem);
        }

        if let Some(filter) = self.filter {
            let filter_elem = Element::new("Filter")
                .set_namespace(WSMAN_NAMESPACE)
                .set_text(filter);

            element = element.add_child(filter_elem);
        }

        element
    }
}

#[derive(Debug, Clone)]
pub struct PullValue<'a> {
    pub enumeration_context: &'a str,
    pub max_elements: Option<u32>,
}

impl<'a> PullValue<'a> {
    pub fn new(enumeration_context: &'a str) -> Self {
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
        let mut element = Element::new(name).set_namespace_optional(namespace.or(Some("http://schemas.xmlsoap.org/ws/2004/09/enumeration")));

        let context_elem = Element::new("EnumerationContext")
            .set_namespace("http://schemas.xmlsoap.org/ws/2004/09/enumeration")
            .set_text(self.enumeration_context);

        element = element.add_child(context_elem);

        if let Some(max) = self.max_elements {
            let max_elem = Element::new("MaxElements")
                .set_namespace(wsman_ns!())
                .set_text_owned(max.to_string());

            element = element.add_child(max_elem);
        }

        element
    }
}

#[derive(Debug, Clone)]
pub struct ReleaseValue<'a> {
    pub enumeration_context: &'a str,
}

impl<'a> ReleaseValue<'a> {
    pub fn new(enumeration_context: &'a str) -> Self {
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
            .set_namespace_optional(namespace.or(Some("http://schemas.xmlsoap.org/ws/2004/09/enumeration")))
            .add_child(context_elem)
    }
}

#[derive(Debug, Clone)]
pub struct GetStatusValue<'a> {
    pub enumeration_context: &'a str,
}

impl<'a> GetStatusValue<'a> {
    pub fn new(enumeration_context: &'a str) -> Self {
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
            .set_namespace_optional(namespace.or(Some("http://schemas.xmlsoap.org/ws/2004/09/enumeration")))
            .add_child(context_elem)
    }
}

// Main body builder structure
#[derive(typed_builder::TypedBuilder, Debug, Clone)]
pub struct WsManagementBody<'a> {
    #[builder(default, setter(into))]
    pub identify: Option<Tag<'a, (), Identify>>,
    #[builder(default, setter(into))]
    pub get: Option<Tag<'a, &'a str, Get>>,
    #[builder(default, setter(into))]
    pub put: Option<Tag<'a, &'a str, Put>>,
    #[builder(default, setter(into))]
    pub create: Option<Tag<'a, &'a str, Create>>,
    #[builder(default, setter(into))]
    pub delete: Option<Tag<'a, &'a str, Delete>>,
    #[builder(default, setter(into))]
    pub rename: Option<Tag<'a, &'a str, Delete>>,
    #[builder(default, setter(into))]
    pub enumerate: Option<Tag<'a, EnumerateValue<'a>, Enumerate>>,
    #[builder(default, setter(into))]
    pub pull: Option<Tag<'a, PullValue<'a>, Pull>>,
    #[builder(default, setter(into))]
    pub release: Option<Tag<'a, ReleaseValue<'a>, Release>>,
    #[builder(default, setter(into))]
    pub get_status: Option<Tag<'a, GetStatusValue<'a>, GetStatus>>,
}

impl<'a> IntoIterator for WsManagementBody<'a> {
    type Item = Element<'a>;
    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        let WsManagementBody {
            identify,
            get,
            put,
            create,
            delete,
            rename,
            enumerate,
            pull,
            release,
            get_status,
        } = self;

        let mut elements = Vec::new();

        if let Some(identify) = identify {
            elements.push(identify.into());
        }
        if let Some(get) = get {
            elements.push(get.into());
        }
        if let Some(put) = put {
            elements.push(put.into());
        }

        if let Some(create) = create {
            elements.push(create.into());
        }
        if let Some(delete) = delete {
            elements.push(delete.into());
        }
        if let Some(rename) = rename {
            elements.push(rename.into());
        }
        if let Some(enumerate) = enumerate {
            elements.push(enumerate.into());
        }
        if let Some(pull) = pull {
            elements.push(pull.into());
        }
        if let Some(release) = release {
            elements.push(release.into());
        }
        if let Some(get_status) = get_status {
            elements.push(get_status.into());
        }

        elements.into_iter()
    }
}

impl<'a> crate::soap::SoapBodys<'a> for WsManagementBody<'a> {
    const NAMESPACE: &'static str = WSMAN_NAMESPACE;
}
