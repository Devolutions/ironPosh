use ironposh_macros::{FromXml, SimpleTagValue};

use crate::cores::Address;
use crate::tag;

tag!(ReplyTo = AddressValue<'a> => WsAddressing2004);

#[derive(Debug, Clone, SimpleTagValue, FromXml)]
pub struct AddressValue<'a> {
    pub url: Address<'a>,
}
