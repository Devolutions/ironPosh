
pub struct Soap<'a> {
    pub ws_addressing_header: Option<crate::ws_addressing::WsAddressingHeaders<'a>>,
    pub ws_management_header: Option<crate::ws_management::WsManagementHeader<'a>>,

    __phantom: std::marker::PhantomData<&'a ()>,
}
