use crate::cores::IdleTimeOut;
use crate::tag;
use ironposh_macros::{FromXml, SimpleTagValue};

tag!(Disconnect = DisconnectValue<'a> => WsmanShell);

/// Value for the Disconnect element (MS-WSMV 3.1.4.13).
/// Optionally carries an IdleTimeOut serialized as `PT{seconds}S`.
#[derive(Debug, Clone, typed_builder::TypedBuilder, SimpleTagValue, FromXml)]
pub struct DisconnectValue<'a> {
    #[builder(default, setter(strip_option, into))]
    pub idle_time_out: Option<IdleTimeOut<'a>>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use ironposh_xml::mapping::FromXml as _;
    use ironposh_xml::parser::parse;

    const RSP: &str = "http://schemas.microsoft.com/wbem/wsman/1/windows/shell";

    #[test]
    fn from_xml_reads_idle_timeout_namespace_correctly() {
        // IdleTimeOut under the correct (rsp) namespace — and with a *different*
        // prefix than the canonical one, to prove identity is by URI, not prefix.
        let xml = format!(
            r#"<x:Disconnect xmlns:x="{RSP}"><x:IdleTimeOut>PT60.000S</x:IdleTimeOut></x:Disconnect>"#
        );
        let doc = parse(&xml).unwrap();
        let value = DisconnectValue::from_xml(doc.root_element()).unwrap();

        let secs = value.idle_time_out.expect("idle_time_out present").value.0;
        assert!((secs - 60.0).abs() < 1e-9);
    }

    #[test]
    fn from_xml_ignores_idle_timeout_in_wrong_namespace() {
        // Same local name, wrong namespace URI — must not bind.
        let xml = r#"<x:Disconnect xmlns:x="http://schemas.microsoft.com/wbem/wsman/1/windows/shell" xmlns:o="http://example.com/other"><o:IdleTimeOut>PT60.000S</o:IdleTimeOut></x:Disconnect>"#;
        let doc = parse(xml).unwrap();
        let value = DisconnectValue::from_xml(doc.root_element()).unwrap();

        assert!(value.idle_time_out.is_none());
    }
}
