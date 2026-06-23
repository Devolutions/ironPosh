pub mod body;
pub mod fault;
pub mod header;
pub mod parsing;

use ironposh_macros::FromXml;

use crate::{
    cores::{Tag, TagValue, tag_name::*},
    soap::{body::SoapBody, header::SoapHeaders},
};

#[derive(Debug, Clone, typed_builder::TypedBuilder, FromXml)]
pub struct SoapEnvelope<'a> {
    #[builder(default, setter(into, strip_option))]
    pub header: Option<Tag<'a, SoapHeaders<'a>, Header>>,
    #[builder(setter(into))]
    pub body: Tag<'a, SoapBody<'a>, Body>,
}

impl<'a> TagValue<'a> for SoapEnvelope<'a> {
    fn append_to_element(
        self,
        element: ironposh_xml::builder::Element<'a>,
    ) -> ironposh_xml::builder::Element<'a> {
        let envelope = element;

        if let Some(header) = self.header {
            envelope.add_child(header.into_element())
        } else {
            envelope
        }
        .add_child(self.body.into_element())
    }
}
