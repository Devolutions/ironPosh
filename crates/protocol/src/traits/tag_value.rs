use xml::builder::Element;

pub trait TagValue<'a> {
    fn into_element(self, name: &'static str, namespace: Option<&'static str>) -> Element<'a>;
}

impl<'a> TagValue<'a> for &'a str {
    fn into_element(self, name: &'static str, namespace: Option<&'static str>) -> Element<'a> {
        let mut element = Element::new(name).set_text(self);
        if let Some(ns) = namespace {
            element = element.set_namespace(ns);
        }
        element
    }
}

impl<'a> TagValue<'a> for () {
    fn into_element(self, name: &'static str, namespace: Option<&'static str>) -> Element<'a> {
        let mut element = Element::new(name);

        if let Some(ns) = namespace {
            element = element.set_namespace(ns);
        }

        element
    }
}

