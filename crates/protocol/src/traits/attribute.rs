pub trait Attribute<'a> {
    fn name(&self) -> &'static str;
    fn value(&self) -> &'a str;
    fn namespace(&self) -> Option<&'static str>;
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
    fn name(&self) -> &'static str {
        "mustUnderstand"
    }

    fn value(&self) -> &'a str {
        if self.value { "true" } else { "false" }
    }

    fn namespace(&self) -> Option<&'static str> {
        Some(crate::soap::SOAP_NAMESPACE)
    }
}
