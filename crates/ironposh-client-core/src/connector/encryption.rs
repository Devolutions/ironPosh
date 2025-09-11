use std::fmt::Debug;

use tracing::{debug, info, instrument};

use crate::{
    PwshCoreError,
    connector::{
        auth_sequence::AuthContext,
        authenticator::SspiAuthenticator,
        http::{ENCRYPTION_BOUNDARY, HttpBody},
    },
};

#[derive(Debug)]
pub struct EncryptionProvider {
    context: AuthContext,
    sequence_number: u32,
    recv_sequence_number: u32,
    require_encryption: bool,
}

#[derive(Debug)]
pub enum EncryptionResult {
    Encrypted { token: Vec<u8> },
    EncryptionNotPerformed,
}

#[derive(Debug)]
pub enum DecryptionResult {
    Decrypted(Vec<u8>),
    DecryptionNotPerformed,
}

impl EncryptionProvider {
    pub fn new(context: AuthContext, require_encryption: bool) -> Self {
        Self {
            context,
            sequence_number: 0,
            recv_sequence_number: 0,
            require_encryption,
        }
    }

    fn next_sequence_number(&mut self) -> u32 {
        let seq = self.sequence_number;
        self.sequence_number = self.sequence_number.wrapping_add(1);
        seq
    }

    fn next_recv_sequence_number(&mut self) -> u32 {
        let seq = self.recv_sequence_number;
        self.recv_sequence_number = self.recv_sequence_number.wrapping_add(1);
        seq
    }

    /// High-level method to encrypt a string into an HttpBody
    #[instrument(skip(self, data))]
    pub fn encrypt(&mut self, data: String) -> Result<HttpBody, PwshCoreError> {
        let sequence_number = self.next_sequence_number();
        // Exact UTF-8 byte length of the ORIGINAL SOAP prior to sealing
        let plain_len = data.len();

        debug!(
            xml_to_encrypt = data.as_str(),
            data_len = plain_len,
            "Encrypting XML body"
        );

        // Keep `data` intact so we can return it if wrap is skipped; encrypt a copy of the bytes
        let mut sealed_bytes = data.as_bytes().to_vec();

        // Perform SSPI/NTLM sealing
        let token = match self.wrap(&mut sealed_bytes, sequence_number)? {
            EncryptionResult::Encrypted { token } => token,
            EncryptionResult::EncryptionNotPerformed => {
                debug!("Encryption not performed, returning original XML body");
                return Ok(HttpBody::Xml(data));
            }
        };

        // 4-byte little-endian length prefix for the verifier token
        let token_len_le = (token.len() as u32).to_le_bytes();

        // (Capacity hint only)
        let body_len = 128 + 64 + token.len() + sealed_bytes.len() + ENCRYPTION_BOUNDARY.len() * 4;

        debug!(
            encrypted_len = sealed_bytes.len(),
            token_len = token.len(),
            body_len,
            "Assembling encrypted HTTP body"
        );

        // Assemble the multipart/encrypted body EXACTLY (CRLF everywhere)
        let mut body: Vec<u8> = Vec::with_capacity(body_len);

        // Part 1 — metadata only
        write_str(&mut body, "--");
        write_str(&mut body, ENCRYPTION_BOUNDARY);
        write_crlf(&mut body);
        write_str(
            &mut body,
            "Content-Type: application/HTTP-SPNEGO-session-encrypted",
        );
        write_crlf(&mut body);
        write_str(
            &mut body,
            "OriginalContent: type=application/soap+xml;charset=UTF-8;Length=",
        );
        write_str(&mut body, &plain_len.to_string());
        write_crlf(&mut body); // end of headers for part 1 (no body)

        // Part 2 — binary payload (security trailer + ciphertext)
        write_str(&mut body, "--");
        write_str(&mut body, ENCRYPTION_BOUNDARY);
        write_crlf(&mut body);
        write_str(&mut body, "Content-Type: application/octet-stream");
        // IMPORTANT: do NOT add Content-Transfer-Encoding
        write_crlf(&mut body); // end of headers for part 2

        // 4-byte length + token + sealed data
        body.extend_from_slice(&token_len_le);
        body.extend_from_slice(&token);
        body.extend_from_slice(&sealed_bytes);

        // Closing boundary (no extra CRLF before it)
        write_str(&mut body, "--");
        write_str(&mut body, ENCRYPTION_BOUNDARY);
        write_str(&mut body, "--");
        write_crlf(&mut body);

        Ok(HttpBody::Encrypted(body))
    }

    /// High-level method to decrypt an HttpBody into a string
    #[instrument(skip(self, data))]
    pub fn decrypt(&mut self, data: HttpBody) -> Result<String, PwshCoreError> {
        let sequence_number = self.next_recv_sequence_number();
        info!(
            body_type = ?data,
            "Decrypting HTTP body if necessary"
        );
        let encrypted_data = match data {
            HttpBody::Encrypted(encrypted_data) => {
                debug!(
                    encrypted_data_len = encrypted_data.len(),
                    "Processing encrypted HTTP body"
                );
                encrypted_data
            }
            _ => {
                debug!("Body is not encrypted, returning as-is");
                return Ok(data.as_str()?.to_owned());
            }
        };

        // Parse the multipart/encrypted body to extract the binary payload
        let binary_payload = extract_binary_payload(&encrypted_data)?;

        debug!(
            payload_len = binary_payload.len(),
            "Extracted binary payload from multipart body"
        );

        // The binary payload contains: 4-byte length + token + encrypted data
        if binary_payload.len() < 4 {
            return Err(PwshCoreError::InternalError(
                "Binary payload too short to contain length prefix".to_string(),
            ));
        }

        // Read the 4-byte little-endian length prefix
        let token_len = u32::from_le_bytes([
            binary_payload[0],
            binary_payload[1],
            binary_payload[2],
            binary_payload[3],
        ]) as usize;

        debug!(token_len, "Read token length from binary payload");

        if binary_payload.len() < 4 + token_len {
            return Err(PwshCoreError::InternalError(format!(
                "Binary payload too short: expected at least {} bytes, got {}",
                4 + token_len,
                binary_payload.len()
            )));
        }

        // Extract the token (security trailer)
        let token = &binary_payload[4..4 + token_len];
        debug!(
            token_len = token.len(),
            token_bytes = ?&token[..token.len().min(16)].iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>(),
            "Extracted security token"
        );

        // Extract the encrypted data (skip 4-byte length prefix + token)
        let encrypted_data_start = 4 + token_len;
        let mut encrypted_data = binary_payload[encrypted_data_start..].to_vec();

        debug!(
            encrypted_data_len = encrypted_data.len(),
            encrypted_bytes = ?&encrypted_data[..encrypted_data.len().min(16)],
            "Extracted encrypted data for decryption"
        );

        match self.unwrap(token, &mut encrypted_data, sequence_number)? {
            DecryptionResult::Decrypted(items) => {
                let decrypted = String::from_utf8(items).map_err(|e| {
                    PwshCoreError::InternalError(format!("Failed to decode decrypted body: {e}"))
                })?;

                debug!(
                    xml_decrypted = decrypted.as_str(),
                    decrypted_len = decrypted.len(),
                    "Successfully decrypted XML body"
                );

                Ok(decrypted)
            }
            DecryptionResult::DecryptionNotPerformed => {
                Err(PwshCoreError::InvalidState("Decryption was not performed"))
            }
        }
    }

    /// Extract the binary payload from a multipart/encrypted HTTP body

    fn wrap(
        &mut self,
        data: &mut [u8],
        sequence_number: u32,
    ) -> Result<EncryptionResult, PwshCoreError> {
        if !self.require_encryption {
            debug!("Encryption not required, skipping wrap");
            return Ok(EncryptionResult::EncryptionNotPerformed);
        }

        let token = match &mut self.context {
            AuthContext::Ntlm(auth_context) => {
                SspiAuthenticator::wrap(&mut auth_context.provider, data, sequence_number)
            }
            AuthContext::Kerberos(auth_context) => {
                SspiAuthenticator::wrap(&mut auth_context.provider, data, sequence_number)
            }
            AuthContext::Negotiate(auth_context) => {
                SspiAuthenticator::wrap(&mut auth_context.provider, data, sequence_number)
            }
        }?;

        Ok(EncryptionResult::Encrypted { token })
    }

    fn unwrap(
        &mut self,
        token: &[u8],
        data: &mut [u8],
        sequence_number: u32,
    ) -> Result<DecryptionResult, PwshCoreError> {
        if !self.require_encryption {
            debug!("Decryption not required, skipping unwrap");
            return Ok(DecryptionResult::DecryptionNotPerformed);
        }

        let decrypted = match &mut self.context {
            AuthContext::Ntlm(auth_context) => {
                SspiAuthenticator::unwrap(&mut auth_context.provider, token, data, sequence_number)
            }
            AuthContext::Kerberos(auth_context) => {
                SspiAuthenticator::unwrap(&mut auth_context.provider, token, data, sequence_number)
            }
            AuthContext::Negotiate(auth_context) => {
                SspiAuthenticator::unwrap(&mut auth_context.provider, token, data, sequence_number)
            }
        }?;

        Ok(DecryptionResult::Decrypted(decrypted))
    }
}

#[instrument(skip(data), fields(data_len = data.len()))]
fn extract_binary_payload(data: &[u8]) -> Result<Vec<u8>, PwshCoreError> {
    debug!("Starting multipart body parsing");

    // Convert to string for logging multipart structure
    let data_str = String::from_utf8_lossy(data);
    debug!(
        multipart_preview = %data_str.chars().take(200).collect::<String>(),
        "Multipart body preview"
    );

    // Be a little tolerant of different formats:
    let header_patterns: &[&[u8]] = &[
        b"Content-Type: application/octet-stream",
        b"Content-Type:application/octet-stream",
    ];

    for (pattern_idx, pattern) in header_patterns.iter().enumerate() {
        debug!(
            pattern_idx = pattern_idx,
            pattern = %String::from_utf8_lossy(pattern),
            "Searching for header pattern"
        );

        let Some(header_pos) = find_subsequence(data, pattern) else {
            debug!(pattern_idx = pattern_idx, "Header pattern not found");
            continue;
        };

        debug!(
            pattern_idx = pattern_idx,
            header_pos = header_pos,
            "Found header pattern"
        );

        let mut binary_start = header_pos + pattern.len();

        binary_start += "\r\n".len(); // CRLF after header

        // Find the next boundary to determine where the binary data ends
        let boundary = format!("--{ENCRYPTION_BOUNDARY}");
        let boundary_bytes = boundary.as_bytes();
        let closing_boundary = format!("--{ENCRYPTION_BOUNDARY}--");
        let closing_boundary_bytes = closing_boundary.as_bytes();

        debug!(
            boundary = %boundary,
            closing_boundary = %closing_boundary,
            binary_start = binary_start,
            "Looking for boundaries to determine binary data range"
        );

        // Look for either the next boundary or the closing boundary
        let binary_end = if let Some(next_boundary_pos) =
            find_subsequence(&data[binary_start..], boundary_bytes)
        {
            debug!(next_boundary_pos = next_boundary_pos, "Found next boundary");
            binary_start + next_boundary_pos
        } else if let Some(closing_pos) =
            find_subsequence(&data[binary_start..], closing_boundary_bytes)
        {
            debug!(closing_pos = closing_pos, "Found closing boundary");
            binary_start + closing_pos
        } else {
            debug!("No boundary found, using end of data");
            data.len()
        };

        if binary_end > binary_start {
            let payload_len = binary_end - binary_start;
            debug!(
                binary_start = binary_start,
                binary_end = binary_end,
                payload_len = payload_len,
                "Successfully extracted binary payload"
            );
            return Ok(data[binary_start..binary_end].to_vec());
        }
    }

    Err(PwshCoreError::InternalError(
        "Could not find binary payload in multipart body".to_string(),
    ))
}

#[inline]
fn write_str(buf: &mut Vec<u8>, s: &str) {
    buf.extend_from_slice(s.as_bytes());
}

#[inline]
fn write_crlf(buf: &mut Vec<u8>) {
    buf.extend_from_slice(b"\r\n");
}

/// Find the position of a subsequence within a byte slice
fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() {
        return Some(0);
    }

    if haystack.len() < needle.len() {
        return None;
    }

    (0..=(haystack.len() - needle.len())).find(|&i| haystack[i..i + needle.len()] == *needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    const REAL_ENCRYPTED_SOAP: &str = include_str!("..\\..\\test_data\\real_encrypted_soap.txt");

    fn hex_decode(hex_str: &str) -> Vec<u8> {
        (0..hex_str.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&hex_str[i..i + 2], 16))
            .collect::<Result<Vec<u8>, _>>()
            .expect("Invalid hex string")
    }

    #[test]
    fn test_extract_binary_payload_from_real_data() {
        // Decode the hex string to bytes
        let encrypted_data = hex_decode(REAL_ENCRYPTED_SOAP.trim());

        println!(
            "Total encrypted data length: {} bytes",
            encrypted_data.len()
        );

        // Try to extract the binary payload using our standalone function
        let result = extract_binary_payload(&encrypted_data);

        match result {
            Ok(binary_payload) => {
                println!(
                    "Successfully extracted binary payload: {} bytes",
                    binary_payload.len()
                );

                // Verify we have at least 4 bytes for the length prefix
                assert!(binary_payload.len() >= 4, "Binary payload too short");

                // Read the token length
                let token_len = u32::from_le_bytes([
                    binary_payload[0],
                    binary_payload[1],
                    binary_payload[2],
                    binary_payload[3],
                ]) as usize;

                println!("Token length: {} bytes", token_len);

                // Verify the structure makes sense
                assert!(token_len > 0, "Token length should be greater than 0");
                assert!(
                    binary_payload.len() >= 4 + token_len,
                    "Payload should contain token + encrypted data"
                );

                let encrypted_data_len = binary_payload.len() - 4 - token_len;
                println!("Encrypted data length: {} bytes", encrypted_data_len);

                // Print first few bytes of each section for debugging
                println!(
                    "First 16 bytes of binary payload: {:02x?}",
                    &binary_payload[..16.min(binary_payload.len())]
                );
                if binary_payload.len() > 4 + token_len {
                    let encrypted_start = 4 + token_len;
                    let encrypted_end = (encrypted_start + 16).min(binary_payload.len());
                    println!(
                        "First 16 bytes of encrypted data: {:02x?}",
                        &binary_payload[encrypted_start..encrypted_end]
                    );
                }
            }
            Err(e) => {
                panic!("Failed to extract binary payload: {}", e);
            }
        }
    }

    #[test]
    fn test_multipart_structure_parsing() {
        let encrypted_data = hex_decode(REAL_ENCRYPTED_SOAP.trim());

        // Convert to string to examine the multipart structure
        let data_str = String::from_utf8_lossy(&encrypted_data);
        println!("Multipart structure:");
        println!("{}", data_str);

        // Look for our expected boundary
        let boundary = format!("--{}", ENCRYPTION_BOUNDARY);
        println!("Looking for boundary: '{}'", boundary);

        let parts: Vec<&str> = data_str.split(&boundary).collect();
        println!("Found {} parts", parts.len());

        for (i, part) in parts.iter().enumerate() {
            println!(
                "Part {}: '{}'",
                i,
                part.chars().take(100).collect::<String>()
            );
        }

        // Look for the octet-stream header
        let octet_stream_header = "Content-Type: application/octet-stream";
        if data_str.contains(octet_stream_header) {
            println!("Found octet-stream header");
        } else {
            println!("octet-stream header NOT found");
        }
    }
}
