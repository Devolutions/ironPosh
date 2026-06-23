//! PSRP session crypto: SecureString encryption and the key-exchange state.
//!
//! MS-PSRP encrypts `SecureString` values with a session key negotiated via
//! `PUBLIC_KEY` / `ENCRYPTED_SESSION_KEY` (AES-256-CBC, zero IV). This module
//! holds that state and the in-place encryption walk; the runspace pool drives
//! the key exchange and calls in here when serializing values.

use aes::Aes256;
use cipher::block_padding::Pkcs7;
use cipher::{BlockModeEncrypt, KeyIvInit};
use tracing::debug;

#[derive(Debug)]
pub(super) struct KeyExchangeState {
    pub(super) private_key: rsa::RsaPrivateKey,
    pub(super) session_key: Option<Vec<u8>>,
}

pub(super) fn encrypt_secure_strings_in_value_rec(
    value: &mut ironposh_psrp::PsValue,
    session_key: Option<&[u8]>,
) -> Result<(), crate::PwshCoreError> {
    use ironposh_psrp::{ComplexObjectContent, Container, PsPrimitiveValue, PsValue};

    match value {
        PsValue::Primitive(PsPrimitiveValue::SecureString(bytes)) => {
            let Some(session_key) = session_key else {
                return Err(crate::PwshCoreError::InvalidResponse(
                    "SecureString encountered but PSRP session key is not established".into(),
                ));
            };
            encrypt_secure_string_bytes_in_place(bytes, session_key)?;
        }
        PsValue::Primitive(_) => {}
        PsValue::Object(obj) => {
            for value in obj.properties.values_mut() {
                encrypt_secure_strings_in_value_rec(value, session_key)?;
            }

            match &mut obj.content {
                ComplexObjectContent::ExtendedPrimitive(p) => {
                    if let PsPrimitiveValue::SecureString(bytes) = p {
                        let Some(session_key) = session_key else {
                            return Err(crate::PwshCoreError::InvalidResponse(
                                "SecureString encountered but PSRP session key is not established"
                                    .into(),
                            ));
                        };
                        encrypt_secure_string_bytes_in_place(bytes, session_key)?;
                    }
                }
                ComplexObjectContent::Container(
                    Container::Stack(items) | Container::Queue(items) | Container::List(items),
                ) => {
                    for item in items.iter_mut() {
                        encrypt_secure_strings_in_value_rec(item, session_key)?;
                    }
                }
                ComplexObjectContent::Container(Container::Dictionary(dict)) => {
                    for (_k, v) in dict.iter_mut() {
                        encrypt_secure_strings_in_value_rec(v, session_key)?;
                    }
                }
                ComplexObjectContent::Standard | ComplexObjectContent::PsEnums(_) => {}
            }
        }
    }

    Ok(())
}

fn encrypt_secure_string_bytes_in_place(
    bytes: &mut Vec<u8>,
    session_key: &[u8],
) -> Result<(), crate::PwshCoreError> {
    if session_key.len() != 32 {
        return Err(crate::PwshCoreError::InvalidResponse(
            format!(
                "PSRP SecureString encryption requires 32-byte session key; got {}",
                session_key.len()
            )
            .into(),
        ));
    }

    // PowerShell's PSRP SecureString encryption uses AES-256-CBC with a zero IV.
    // The <SS> payload is the ciphertext bytes only (base64 encoded).
    let iv = [0u8; 16];

    let encryptor = cbc::Encryptor::<Aes256>::new_from_slices(session_key, &iv).map_err(|e| {
        crate::PwshCoreError::InvalidResponse(
            format!("Failed to initialize AES encryptor: {e}").into(),
        )
    })?;

    // MS-PSRP SecureString payload is UTF-16LE plaintext encrypted with AES-256-CBC.
    let msg_len = bytes.len();
    let pad = 16 - (msg_len % 16);
    let mut buf = bytes.clone();
    buf.resize(msg_len + pad, 0);
    let ciphertext = encryptor
        .encrypt_padded::<Pkcs7>(&mut buf, msg_len)
        .map_err(|e| {
            crate::PwshCoreError::InvalidResponse(
                format!("Failed to encrypt SecureString (padding): {e}").into(),
            )
        })?;

    let out = ciphertext.to_vec();

    debug!(
        session_key_len = session_key.len(),
        plaintext_len = msg_len,
        encrypted_len = out.len(),
        "encrypted SecureString payload"
    );

    *bytes = out;
    Ok(())
}
