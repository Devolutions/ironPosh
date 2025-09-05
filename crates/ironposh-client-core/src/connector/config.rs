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
        target_name: String,
        identity: crate::credentials::ClientAuthIdentity,
    },
    Kerberos {
        target_name: String,
        identity: crate::credentials::ClientAuthIdentity,
        kerberos_config: KerberosConfig,
    },
    Negotiate {
        target_name: String,
        identity: crate::credentials::ClientAuthIdentity,
        kerberos_config: Option<KerberosConfig>,
    },
}

impl SspiAuthConfig {

    pub(crate) fn target_name(&self) -> &str {
        match self {
            SspiAuthConfig::NTLM { target_name, .. } => target_name,
            SspiAuthConfig::Kerberos { target_name, .. } => target_name,
            SspiAuthConfig::Negotiate { target_name, .. } => target_name,
        }
    }
    
}

#[derive(Debug, Clone)]
pub enum Authentication {
    Basic { username: String, password: String },

    Sspi(SspiAuthConfig),
}
