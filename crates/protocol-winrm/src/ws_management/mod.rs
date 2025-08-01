pub mod body;
pub mod header;
pub use header::*;

use crate::{
    cores::{Attribute, Tag, Time, anytag::AnyTag, namespace::Namespace, tag_name::*},
    soap::{SoapEnvelope, body::SoapBody, header::SoapHeaders},
    ws_addressing::AddressValue,
};

#[derive(Debug, Clone, typed_builder::TypedBuilder)]
pub struct WsMan {
    #[builder(default = 143600)]
    max_envelope_size: u32,

    #[builder(default = 20)]
    operation_timeout: u32,

    #[builder(default = "eb-CA".to_string())]
    data_locale: String,

    #[builder(default = "en-US".to_string())]
    locale: String,

    #[builder(default = "http://schemas.microsoft.com/wbem/wsman/1/windows/shell/cmd".to_string())]
    resource_uri: String,
}

impl WsMan {
    pub fn max_envelope_size(&self) -> u32 {
        self.max_envelope_size
    }
}

#[derive(Debug, Clone)]
pub enum WsAction {
    Create,
    Delete,
    Get,
    Put,
}

impl WsAction {
    pub fn as_str(&self) -> &str {
        match self {
            WsAction::Create => "http://schemas.dmtf.org/wbem/wsman/1/wsman/Create",
            WsAction::Delete => "http://schemas.dmtf.org/wbem/wsman/1/wsman/Delete",
            WsAction::Get => "http://schemas.dmtf.org/wbem/wsman/1/wsman/Get",
            WsAction::Put => "http://schemas.dmtf.org/wbem/wsman/1/wsman/Put",
        }
    }
}

impl WsMan {
    pub fn invoke<'a>(
        &'a self,
        action: WsAction,
        resource_uri: Option<&'a str>,
        resource: Option<AnyTag<'a>>,
        option_set: Option<header::OptionSetValue>,
        selector_set: Option<header::SelectorSetValue>,
    ) -> Tag<'a, SoapEnvelope<'a>, Envelope> {
        // Generate a unique message ID
        let message_id = uuid::Uuid::new_v4();

        let resource_uri = resource_uri.unwrap_or(self.resource_uri.as_str());

        // Create reply-to address value
        let reply_to_addr = AddressValue {
            url: Tag::new("http://schemas.xmlsoap.org/ws/2004/08/addressing/role/anonymous"),
        };

        // Create the SOAP header with all required fields
        let header = SoapHeaders::builder()
            .action(
                Tag::new(action.as_str().to_owned())
                    .with_name(Action)
                    .with_attribute(Attribute::MustUnderstand(true)),
            )
            .data_locale(
                Tag::new(())
                    .with_attribute(Attribute::MustUnderstand(false))
                    .with_attribute(Attribute::XmlLang(self.data_locale.clone().into())), //
            )
            .locale(
                Tag::new(())
                    .with_attribute(Attribute::XmlLang(self.locale.clone().into()))
                    .with_attribute(Attribute::MustUnderstand(false)),
            )
            .max_envelope_size(
                Tag::new(self.max_envelope_size).with_attribute(Attribute::MustUnderstand(true)),
            )
            .resource_uri(resource_uri)
            .operation_timeout(Time::from(self.operation_timeout))
            .message_id(message_id)
            .to("http://schemas.xmlsoap.org/ws/2004/08/addressing/role/anonymous")
            .reply_to(reply_to_addr)
            .option_set_opt(option_set.map(Tag::from))
            .selector_set_opt(selector_set.map(Tag::from))
            .build();

        // TODO: Handle the case where resource is something else
        let body = match resource {
            Some(AnyTag::Shell(shell)) => SoapBody::builder().shell(shell).build(),
            _ => SoapBody::builder().build(),
        };

        // Create the complete SOAP envelope
        let envelope = SoapEnvelope::builder().header(header).body(body).build();

        // Convert to XML using Tag wrapper with proper namespaces
        let envelope_tag = Tag::<SoapEnvelope, Envelope>::new(envelope)
            .with_declaration(Namespace::SoapEnvelope2003)
            .with_declaration(Namespace::WsAddressing2004)
            .with_declaration(Namespace::DmtfWsmanSchema)
            .with_declaration(Namespace::MsWsmanSchema)
            .with_declaration(Namespace::WsmanShell);

        envelope_tag
    }
}
