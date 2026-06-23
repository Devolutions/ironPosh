use ironposh_macros::{PsDeserialize, PsSerialize};

/// Client → Server public key used for PSRP session key exchange.
///
/// ```xml
/// <Obj RefId="0">
///   <MS>
///     <S N="PublicKey">...base64...</S>
///   </MS>
/// </Obj>
/// ```
#[derive(Debug, Clone, PartialEq, Eq, PsSerialize, PsDeserialize)]
#[ps(message_type = PublicKey)]
pub struct PublicKey {
    /// Base64-encoded public key blob as defined by MS-PSRP.
    #[ps(name = "PublicKey")]
    pub public_key: String,
}
