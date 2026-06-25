pub mod body;
pub mod fault;
pub mod header;
pub mod parsing;

use ironposh_macros::FromXml;

use crate::cores::TagValue;
use crate::tag;
use crate::{soap::body::Body, soap::header::Header};

tag!(Envelope = SoapEnvelope<'a> => SoapEnvelope2003);

#[derive(Debug, Clone, typed_builder::TypedBuilder, FromXml)]
pub struct SoapEnvelope<'a> {
    #[builder(default, setter(into, strip_option))]
    pub header: Option<Header<'a>>,
    #[builder(setter(into))]
    pub body: Body<'a>,
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
