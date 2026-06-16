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
    /// SSPI authentication (NTLM, Kerberos, or Negotiate).
    /// Note: SSPI message sealing is now controlled by `TransportSecurity` in `WinRmConfig`.
    Sspi(SspiAuthConfig),
}

/// TLS behaviour for HTTPS transports. Honored by `HttpClient` implementations
/// (reqwest-based clients); ignored for plain-HTTP transports and for the WASM
/// client (the browser owns TLS there).
#[derive(Debug, Clone, Default)]
pub struct TlsOptions {
    /// Accept any server certificate (self-signed labs). DANGEROUS outside test/lab use.
    pub accept_invalid_certs: bool,
    /// Skip hostname verification only.
    pub accept_invalid_hostnames: bool,
    /// Additional root CA certificate, PEM-encoded. Must contain a single
    /// certificate; PEM bundles (multiple certificates) are not supported.
    pub extra_ca_pem: Option<Vec<u8>>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tls_options_default_is_secure() {
        let tls = TlsOptions::default();
        assert!(!tls.accept_invalid_certs);
        assert!(!tls.accept_invalid_hostnames);
        assert!(tls.extra_ca_pem.is_none());
    }
}
