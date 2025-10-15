use url::Url;

#[derive(Debug, Clone)]
pub struct KerberosConfig {
    /// Optional KDC URL. If not set, the KDC will be discovered via DNS SRV records.
    pub kdc_url: Option<Url>,

    /// Optional client computer name. If not set, the local computer name will be used.
    pub client_computer_name: String,
}

impl From<KerberosConfig> for sspi::KerberosConfig {
    fn from(val: KerberosConfig) -> Self {
        Self {
            kdc_url: val.kdc_url,
            client_computer_name: Some(val.client_computer_name),
        }
    }
}

#[derive(Debug, Clone)]
pub enum SspiAuthConfig {
    NTLM {
        target: String,
        identity: crate::credentials::ClientAuthIdentity,
    },
    Kerberos {
        target: String,
        identity: crate::credentials::ClientAuthIdentity,
        kerberos_config: KerberosConfig,
    },
    Negotiate {
        target: String,
        identity: crate::credentials::ClientAuthIdentity,
        kerberos_config: Option<KerberosConfig>,
    },
}

#[derive(Debug, Clone)]
pub enum AuthenticatorConfig {
    Basic {
        username: String,
        password: String,
    },
    Sspi {
        sspi: SspiAuthConfig,
        /// SSPI message sealing (wrap/unwrap). TLS is separate at transport level.
        require_encryption: bool,
    },
}
