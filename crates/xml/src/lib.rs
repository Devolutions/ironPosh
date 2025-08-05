use roxmltree::NodeType;

pub mod builder;
pub mod parser;

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum XmlError {
    #[error("Invalid XML: {0}")]
    ParserError(#[from] crate::parser::Error),

    #[error("Invalid namespace: expected '{expected}', found '{found:?}'")]
    XmlInvalidNamespace {
        expected: String,
        found: Option<String>,
    },

    #[error("Invalid tag: expected '{expected}', found '{found:?}'")]
    XmlInvalidTag { expected: String, found: String },

    #[error("Invalid number of tags for {tag}: found {value}")]
    TagCountInvalid { tag: String, value: usize },

    #[error("Invalid XML: {0}")]
    InvalidXml(String),

    #[error("{0}")]
    GenericError(String),

    #[error("Unexpected tag: {0}")]
    UnexpectedTag(String),

    #[error("Invalid node type: expected '{expected:?}', found {found:?}")]
    InvalidNodeType { expected: NodeType, found: NodeType },
}
