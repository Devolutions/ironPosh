pub mod header;
pub mod parsing;

use crate::traits::{Tag, TagList, tag_name::*};

pub struct SoapEnvelope<'a> {
    header: Tag<'a, TagList<'a>, Header>,
    body: Tag<'a, TagList<'a>, Body>,
}
