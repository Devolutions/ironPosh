use url::Url;

#[derive(Debug, Clone)]
pub struct KerberosConfig {
    /// Optional KDC URL. If not set, the KDC will be discovered via DNS SRV records.
    pub kdc_url: Option<Url>,

    /// Optional client computer name. If not set, the local computer name will be used.
    pub client_computer_name: Option<String>,
}

#[derive(Debug, Clone)]
pub enum SspiAuthConfig {
    NTLM {
        identity: crate::credentials::ClientAuthIdentity,
    },
    Kerberos {
        identity: crate::credentials::ClientAuthIdentity,
        kerberos_config: KerberosConfig,
    },
    Negotiate {
        identity: crate::credentials::ClientAuthIdentity,
        kerberos_config: Option<KerberosConfig>,
    },
}

#[derive(Debug, Clone)]
pub enum Authentication {
    Basic { username: String, password: String },

    Sspi(SspiAuthConfig),
}
