pub trait Attribute<'a> {
    fn value(&self) -> Option<&'a str>;

    const NAME: &'static str;
    const NAMESPACE: Option<&'static str>;
}

#[derive(Debug, Clone)]
pub struct MustUnderstand {
    pub value: bool,
}

impl MustUnderstand {
    pub fn yes() -> Self {
        MustUnderstand { value: true }
    }

    pub fn no() -> Self {
        MustUnderstand { value: false }
    }
}

impl<'a> Attribute<'a> for MustUnderstand {
    const NAME: &'static str = "mustUnderstand";
    const NAMESPACE: Option<&'static str> = Some(crate::soap::SOAP_NAMESPACE);

    fn value(&self) -> Option<&'a str> {
        if self.value { Some("true") } else { None }
    }
}

