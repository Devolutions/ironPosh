//! Tests for handling malformed SOAP responses.
//!
//! These tests verify that the parser handles invalid/unexpected input gracefully
//! without panicking, returning appropriate errors instead.

use ironposh_winrm::soap::SoapEnvelope;
use ironposh_xml::mapping::FromXml;
use std::fs;

#[cfg(test)]
mod tests {
    use super::*;

    /// Test: XML that is just a Body without Envelope wrapper
    /// Expected: Should fail to parse as SoapEnvelope (wrong root element)
    #[test]
    fn test_missing_envelope_wrapper() {
        let path = "tests/resources/malformed/missing_envelope.xml";
        let xml_content = fs::read_to_string(path).expect("Failed to read file");

        let document = match ironposh_xml::parser::parse(&xml_content) {
            Ok(doc) => doc,
            Err(e) => {
                println!("XML parse failed (acceptable): {e}");
                return;
            }
        };

        let root = document.root_element();
        let result = SoapEnvelope::from_xml(root);

        assert!(
            result.is_err(),
            "Parsing Body without Envelope should fail, got: {result:?}"
        );

        println!(
            "Expected error for missing envelope: {:?}",
            result.unwrap_err()
        );
    }

    /// Test: Valid envelope structure but with empty Body element
    /// Expected: Should parse successfully but body fields should be None/empty
    #[test]
    fn test_empty_body_element() {
        let path = "tests/resources/malformed/empty_body.xml";
        let xml_content = fs::read_to_string(path).expect("Failed to read file");

        let document = ironposh_xml::parser::parse(&xml_content).expect("Valid XML should parse");

        let root = document.root_element();
        let result = SoapEnvelope::from_xml(root);

        // Empty body is technically valid XML - it should parse
        // The resulting SoapEnvelope should have empty body fields
        match result {
            Ok(envelope) => {
                let body = envelope.body.as_ref();
                assert!(
                    body.receive_response.is_none(),
                    "Empty body should have no ReceiveResponse"
                );
                assert!(
                    body.resource_created.is_none(),
                    "Empty body should have no ResourceCreated"
                );
                assert!(body.shell.is_none(), "Empty body should have no Shell");
                println!("Empty body parsed successfully with no content");
            }
            Err(e) => {
                // Also acceptable if implementation requires body content
                println!("Empty body rejected with error: {e}");
            }
        }
    }

    /// Test: Envelope with Header but no Body element at all
    /// Expected: Should either fail or return envelope with default/empty body
    #[test]
    fn test_missing_body_element() {
        let path = "tests/resources/malformed/missing_body.xml";
        let xml_content = fs::read_to_string(path).expect("Failed to read file");

        let document = ironposh_xml::parser::parse(&xml_content).expect("Valid XML should parse");

        let root = document.root_element();
        let result = SoapEnvelope::from_xml(root);

        match result {
            Ok(envelope) => {
                // If it parses, body should be empty/default
                let body = envelope.body.as_ref();
                assert!(body.receive_response.is_none());
                println!("Missing body parsed with default empty body");
            }
            Err(e) => {
                // Failing is also acceptable - Body might be required
                println!("Missing body rejected with error: {e}");
            }
        }
    }

    /// Test: XML truncated in the middle of a tag
    /// Expected: XML parser should fail with syntax error
    #[test]
    fn test_truncated_xml_mid_tag() {
        let path = "tests/resources/malformed/truncated_mid_tag.xml";
        let xml_content = fs::read_to_string(path).expect("Failed to read file");

        let parse_result = ironposh_xml::parser::parse(&xml_content);

        assert!(
            parse_result.is_err(),
            "Truncated XML should fail to parse, got: {parse_result:?}"
        );

        println!(
            "Truncated XML correctly rejected: {:?}",
            parse_result.unwrap_err()
        );
    }

    /// Test: Valid XML structure but with wrong SOAP namespace
    /// Expected: Either parse with degraded functionality or fail
    #[test]
    fn test_wrong_soap_namespace() {
        let path = "tests/resources/malformed/wrong_namespace.xml";
        let xml_content = fs::read_to_string(path).expect("Failed to read file");

        let document =
            ironposh_xml::parser::parse(&xml_content).expect("Valid XML syntax should parse");

        let root = document.root_element();
        let result = SoapEnvelope::from_xml(root);

        // Wrong namespace could either:
        // 1. Fail because the envelope isn't recognized
        // 2. Parse but with missing fields (namespace mismatch)
        match result {
            Ok(envelope) => {
                println!("Wrong namespace parsed (lenient mode): {envelope:?}");
            }
            Err(e) => {
                println!("Wrong namespace correctly rejected: {e}");
            }
        }
    }

    /// Test: Valid SOAP envelope with unknown/extra extension elements mixed in.
    ///
    /// The parser is lenient (SOAP must-ignore semantics): unknown elements and
    /// extension namespaces are ignored, and the known content is still
    /// extracted. Matching is by `(URI, local-name)`, so unrecognized children
    /// simply don't bind to any field.
    #[test]
    fn test_extra_unknown_elements_ignored() {
        let path = "tests/resources/malformed/extra_unknown_elements.xml";
        let xml_content = fs::read_to_string(path).expect("Failed to read file");

        let document = ironposh_xml::parser::parse(&xml_content).expect("Valid XML should parse");
        let envelope = SoapEnvelope::from_xml(document.root_element())
            .expect("Envelope with unknown extension elements should still parse");

        // The known ReceiveResponse/Stream content is extracted despite the noise.
        let receive_response = envelope
            .body
            .as_ref()
            .receive_response
            .as_ref()
            .expect("ReceiveResponse should be parsed");
        assert!(
            !receive_response.as_ref().streams.is_empty(),
            "Stream content should be extracted alongside ignored unknown elements"
        );
    }

    /// Test: XML with mismatched tags (invalid syntax)
    /// Expected: XML parser should fail
    #[test]
    fn test_invalid_xml_syntax_mismatched_tags() {
        let path = "tests/resources/malformed/invalid_xml_syntax.xml";
        let xml_content = fs::read_to_string(path).expect("Failed to read file");

        let parse_result = ironposh_xml::parser::parse(&xml_content);

        assert!(
            parse_result.is_err(),
            "Mismatched tags should fail XML parsing"
        );

        println!(
            "Mismatched tags correctly rejected: {:?}",
            parse_result.unwrap_err()
        );
    }

    /// Test: File that isn't XML at all
    /// Expected: XML parser should fail immediately
    #[test]
    fn test_not_xml_at_all() {
        let path = "tests/resources/malformed/not_xml_at_all.xml";
        let xml_content = fs::read_to_string(path).expect("Failed to read file");

        let parse_result = ironposh_xml::parser::parse(&xml_content);

        assert!(parse_result.is_err(), "Non-XML content should fail parsing");

        println!(
            "Non-XML content correctly rejected: {:?}",
            parse_result.unwrap_err()
        );
    }

    /// Test: Empty string input
    /// Expected: Should fail gracefully
    #[test]
    fn test_empty_input() {
        let parse_result = ironposh_xml::parser::parse("");

        assert!(parse_result.is_err(), "Empty input should fail parsing");

        println!(
            "Empty input correctly rejected: {:?}",
            parse_result.unwrap_err()
        );
    }

    /// Test: Whitespace-only input
    /// Expected: Should fail gracefully
    #[test]
    fn test_whitespace_only_input() {
        let parse_result = ironposh_xml::parser::parse("   \n\t\n   ");

        assert!(
            parse_result.is_err(),
            "Whitespace-only input should fail parsing"
        );

        println!(
            "Whitespace-only input correctly rejected: {:?}",
            parse_result.unwrap_err()
        );
    }
}
