use xml::parser::Node;

use crate::{
    must_be_element, must_be_tag, must_have_namespace,
    soap::{SOAP_NAMESPACE, SoapHeaders},
    ws_addressing::WsAddressingHeaders,
    ws_management::WsManagementHeader,
};

pub struct Soap<'a> {
    pub ws_addressing_header: Option<crate::ws_addressing::WsAddressingHeaders<'a>>,
    pub ws_management_header: Option<crate::ws_management::WsManagementHeader<'a>>,

    __phantom: std::marker::PhantomData<&'a ()>,
}
