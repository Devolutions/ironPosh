use crate::define_tag;

pub mod attribute;
pub mod namespace;
pub mod tag;
pub mod tag_name;
pub mod tag_value;

// Re-export all public items for backward compatibility
pub use attribute::{Attribute, MustUnderstand};
pub use namespace::{DeclareNamespaces, NamespaceWithAlias, PowerShellNamespaceAlias};
pub use tag::Tag;
pub use tag_name::TagName;
pub use tag_value::TagValue;

// Define tag types using the macro
define_tag!(Tag1, (Attribute1, attribute1));
define_tag!(Tag2, (Attribute1, attribute1), (Attribute2, attribute2));
define_tag!(
    Tag3,
    (Attribute1, attribute1),
    (Attribute2, attribute2),
    (Attribute3, attribute3)
);
