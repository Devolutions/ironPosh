#[macro_export]
macro_rules! opt_header {
    ($vec:ident, $($field:expr),* $(,)?) => {
        $(
            if let Some(h) = $field {
                $vec.push(h);
            }
        )*
    };
}

#[macro_export]
macro_rules! must_have_namespace {
    ($element:expr, $namespace:expr) => {
        if !$element
            .tag_name()
            .namespace()
            .is_some_and(|ns| ns == $namespace)
        {
            return Err(xml::XmlError::XmlInvalidNamespace {
                expected: $namespace,
                found: $element.tag_name().namespace(),
            });
        }
    };
}

#[macro_export]
macro_rules! must_be_tag {
    ($element:expr, $tag:expr) => {
        if $element.tag_name().name() != $tag {
            return Err(xml::XmlError::XmlInvalidTag {
                expected: $tag,
                found: $element.tag_name().name(),
            });
        }
    };
}

#[macro_export]
macro_rules! must_be_element {
    ($element:expr) => {
        if !$element.is_element() {
            return Err(xml::XmlError::InvalidNodeType {
                expected: xml::parser::NodeType::Element,
                found: $element.node_type(),
            });
        }
    };
}

#[macro_export]
macro_rules! must_be_root {
    ($element:expr) => {
        if !$element.is_root() {
            return Err(xml::XmlError::InvalidNodeType {
                expected: xml::parser::NodeType::Root,
                found: $element.node_type(),
            });
        }
    };
}

#[macro_export]
macro_rules! must_be_text {
    ($element:expr) => {
        if !$element.is_text() {
            return Err(xml::XmlError::InvalidNodeType {
                expected: xml::parser::NodeType::Text,
                found: $element.node_type(),
            });
        }
    };
}
