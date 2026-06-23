use ironposh_macros::{PsDeserialize, PsSerialize};

/// Server → Client encrypted session key for PSRP session key exchange.
///
/// ```xml
/// <Obj RefId="0">
///   <MS>
///     <S N="EncryptedSessionKey">...base64...</S>
///   </MS>
/// </Obj>
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PsSerialize, PsDeserialize)]
#[ps(message_type = EncryptedSessionKey)]
pub struct EncryptedSessionKey {
    /// Base64-encoded encrypted session key blob as defined by MS-PSRP.
    #[ps(name = "EncryptedSessionKey")]
    pub encrypted_session_key: String,
}
