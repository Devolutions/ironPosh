use crate::soap::{Header, Value};

pub trait MustUnderstand<'a, T>
where
    T: Value<'a>,
{
    fn must_understand(self) -> Header<'a, T>;
}

impl<'a, TNodeValue, THeader> MustUnderstand<'a, TNodeValue> for THeader
where
    TNodeValue: Value<'a>,
    THeader: Into<Header<'a, TNodeValue>>,
{
    fn must_understand(self) -> Header<'a, TNodeValue> {
        let mut header = self.into();
        header.must_understand = true;
        header
    }
}