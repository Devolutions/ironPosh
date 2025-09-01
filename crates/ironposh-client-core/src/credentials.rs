use crate::PwshCoreError;

#[derive(Debug, Clone)]
pub struct ClientUserName {
    inner: sspi::Username,
}

impl ClientUserName {
    pub fn new_upn(account_name: &str, upn_suffix: &str) -> Result<Self, crate::PwshCoreError> {
        let inner = sspi::Username::new_upn(account_name, upn_suffix)
            .map_err(|_| PwshCoreError::UsernameError("failed to create UPN username"))?;
        Ok(Self { inner })
    }

    pub fn new_down_level_logon_name(
        account_name: &str,
        netbios_domain_name: &str,
    ) -> Result<Self, crate::PwshCoreError> {
        let inner = sspi::Username::new_down_level_logon_name(account_name, netbios_domain_name)
            .map_err(|_| PwshCoreError::UsernameError("failed to create down-level logon name"))?;

        Ok(Self { inner })
    }
    pub fn new(
        account_name: &str,
        netbios_domain_name: Option<&str>,
    ) -> Result<Self, crate::PwshCoreError> {
        let inner = match netbios_domain_name {
            Some(netbios_domain_name) if !netbios_domain_name.is_empty() => {
                sspi::Username::new_down_level_logon_name(account_name, netbios_domain_name)
                    .map_err(|_| {
                        PwshCoreError::UsernameError("failed to create down-level logon name")
                    })?
            }
            _ => sspi::Username::parse(account_name)
                .map_err(|_| PwshCoreError::UsernameError("failed to parse username"))?,
        };
        Ok(Self { inner })
    }

    /// Parses the value in order to find if the value is a user principal name or a down-level logon name
    ///
    /// If there is no `\` or `@` separator, the value is considered to be a down-level logon name with
    /// an empty NetBIOS domain.
    pub fn parse(value: &str) -> Result<Self, crate::PwshCoreError> {
        let inner = sspi::Username::parse(value)
            .map_err(|_| PwshCoreError::UsernameError("failed to parse username"))?;
        Ok(Self { inner })
    }

    /// Returns the internal representation, as-is
    pub fn inner(&self) -> &str {
        self.inner.inner()
    }

    /// Returns the username format for this username
    pub fn format(&self) -> sspi::UserNameFormat {
        self.inner.format()
    }

    /// May return an UPN suffix or NetBIOS domain name depending on the internal format
    pub fn domain_name(&self) -> Option<&str> {
        self.inner.domain_name()
    }

    /// Returns the account name
    pub fn account_name(&self) -> &str {
        self.inner.account_name()
    }
}

/// Public wrapper for authentication credentials that hides the sspi AuthIdentity
#[derive(Debug, Clone)]
pub struct ClientAuthIdentity {
    inner: sspi::AuthIdentity,
}

impl ClientAuthIdentity {
    pub fn new(username: ClientUserName, password: String) -> Self {
        let inner = sspi::AuthIdentity {
            username: username.inner,
            password: password.into(),
        };
        Self { inner }
    }

    pub(crate) fn into_inner(self) -> sspi::AuthIdentity {
        self.inner
    }
}
