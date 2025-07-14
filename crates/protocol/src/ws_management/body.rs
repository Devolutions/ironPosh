use xml::builder::Element;

use crate::{soap::Value, ws_management::WSMAN_NAMESPACE, wsman_ns};

pub fn body_builder<'a>() -> WsManagementBodyBuilder<'a> {
    WsManagementBody::builder()
}

// Basic WS-Management operations
#[derive(Debug, Clone)]
pub struct Identify;

impl<'a> Value<'a> for Identify {
    fn into_element(self, name: &'static str) -> Element<'a> {
        Element::new(name).set_namespace(wsman_ns!())
    }
}

#[derive(Debug, Clone)]
pub struct Get;

impl<'a> Value<'a> for Get {
    fn into_element(self, name: &'static str) -> Element<'a> {
        Element::new(name).set_namespace("http://schemas.xmlsoap.org/ws/2004/09/transfer")
    }
}

#[derive(Debug, Clone)]
pub struct Put<'a> {
    pub content: Option<&'a str>,
}

impl<'a> Put<'a> {
    pub fn new() -> Self {
        Self { content: None }
    }

    pub fn with_content(mut self, content: &'a str) -> Self {
        self.content = Some(content);
        self
    }
}

impl<'a> Default for Put<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> Value<'a> for Put<'a> {
    fn into_element(self, name: &'static str) -> Element<'a> {
        let mut element =
            Element::new(name).set_namespace("http://schemas.xmlsoap.org/ws/2004/09/transfer");
        if let Some(content) = self.content {
            element = element.set_text(content);
        }
        element
    }
}

#[derive(Debug, Clone)]
pub struct Create<'a> {
    pub content: Option<&'a str>,
}

impl<'a> Create<'a> {
    pub fn new() -> Self {
        Self { content: None }
    }

    pub fn with_content(mut self, content: &'a str) -> Self {
        self.content = Some(content);
        self
    }
}

impl<'a> Default for Create<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> Value<'a> for Create<'a> {
    fn into_element(self, name: &'static str) -> Element<'a> {
        let mut element =
            Element::new(name).set_namespace("http://schemas.xmlsoap.org/ws/2004/09/transfer");
        if let Some(content) = self.content {
            element = element.set_text(content);
        }
        element
    }
}

#[derive(Debug, Clone)]
pub struct Delete;

impl<'a> Value<'a> for Delete {
    fn into_element(self, name: &'static str) -> Element<'a> {
        Element::new(name).set_namespace("http://schemas.xmlsoap.org/ws/2004/09/transfer")
    }
}

#[derive(Debug, Clone)]
pub struct Rename<'a> {
    pub new_name: &'a str,
}

impl<'a> Rename<'a> {
    pub fn new(new_name: &'a str) -> Self {
        Self { new_name }
    }
}

impl<'a> Value<'a> for Rename<'a> {
    fn into_element(self, name: &'static str) -> Element<'a> {
        Element::new(name)
            .set_namespace(wsman_ns!())
            .set_text(self.new_name)
    }
}

// Enumeration operations
#[derive(Debug, Clone)]
pub struct Enumerate<'a> {
    pub optimize_enumeration: Option<bool>,
    pub max_elements: Option<u32>,
    pub filter: Option<&'a str>,
}

impl<'a> Enumerate<'a> {
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

impl<'a> Default for Enumerate<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> Value<'a> for Enumerate<'a> {
    fn into_element(self, name: &'static str) -> Element<'a> {
        let mut element = Element::new(name).set_namespace(WSMAN_NAMESPACE);

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
pub struct Pull<'a> {
    pub enumeration_context: &'a str,
    pub max_elements: Option<u32>,
}

impl<'a> Pull<'a> {
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

impl<'a> Value<'a> for Pull<'a> {
    fn into_element(self, name: &'static str) -> Element<'a> {
        let mut element =
            Element::new(name).set_namespace("http://schemas.xmlsoap.org/ws/2004/09/enumeration");

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
pub struct Release<'a> {
    pub enumeration_context: &'a str,
}

impl<'a> Release<'a> {
    pub fn new(enumeration_context: &'a str) -> Self {
        Self {
            enumeration_context,
        }
    }
}

impl<'a> Value<'a> for Release<'a> {
    fn into_element(self, name: &'static str) -> Element<'a> {
        let context_elem = Element::new("EnumerationContext")
            .set_namespace("http://schemas.xmlsoap.org/ws/2004/09/enumeration")
            .set_text(self.enumeration_context);

        Element::new(name)
            .set_namespace("http://schemas.xmlsoap.org/ws/2004/09/enumeration")
            .add_child(context_elem)
    }
}

#[derive(Debug, Clone)]
pub struct GetStatus<'a> {
    pub enumeration_context: &'a str,
}

impl<'a> GetStatus<'a> {
    pub fn new(enumeration_context: &'a str) -> Self {
        Self {
            enumeration_context,
        }
    }
}

impl<'a> Value<'a> for GetStatus<'a> {
    fn into_element(self, name: &'static str) -> Element<'a> {
        let context_elem = Element::new("EnumerationContext")
            .set_namespace("http://schemas.xmlsoap.org/ws/2004/09/enumeration")
            .set_text(self.enumeration_context);

        Element::new(name)
            .set_namespace("http://schemas.xmlsoap.org/ws/2004/09/enumeration")
            .add_child(context_elem)
    }
}

// Main body builder structure
#[derive(typed_builder::TypedBuilder, Debug, Clone)]
pub struct WsManagementBody<'a> {
    #[builder(default, setter(into))]
    pub identify: Option<Identify>,
    #[builder(default, setter(into))]
    pub get: Option<Get>,
    #[builder(default, setter(into))]
    pub put: Option<Put<'a>>,
    #[builder(default, setter(into))]
    pub create: Option<Create<'a>>,
    #[builder(default, setter(into))]
    pub delete: Option<Delete>,
    #[builder(default, setter(into))]
    pub rename: Option<Rename<'a>>,
    #[builder(default, setter(into))]
    pub enumerate: Option<Enumerate<'a>>,
    #[builder(default, setter(into))]
    pub pull: Option<Pull<'a>>,
    #[builder(default, setter(into))]
    pub release: Option<Release<'a>>,
    #[builder(default, setter(into))]
    pub get_status: Option<GetStatus<'a>>,
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
            elements.push(identify.into_element("Identify"));
        }
        if let Some(get) = get {
            elements.push(get.into_element("Get"));
        }
        if let Some(put) = put {
            elements.push(put.into_element("Put"));
        }
        if let Some(create) = create {
            elements.push(create.into_element("Create"));
        }
        if let Some(delete) = delete {
            elements.push(delete.into_element("Delete"));
        }
        if let Some(rename) = rename {
            elements.push(rename.into_element("Rename"));
        }
        if let Some(enumerate) = enumerate {
            elements.push(enumerate.into_element("Enumerate"));
        }
        if let Some(pull) = pull {
            elements.push(pull.into_element("Pull"));
        }
        if let Some(release) = release {
            elements.push(release.into_element("Release"));
        }
        if let Some(get_status) = get_status {
            elements.push(get_status.into_element("GetStatus"));
        }

        elements.into_iter()
    }
}

impl<'a> crate::soap::SoapBodys<'a> for WsManagementBody<'a> {
    const NAMESPACE: &'static str = WSMAN_NAMESPACE;
}
