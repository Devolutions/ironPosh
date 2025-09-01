// examples/first_step.rs (or wherever you test)

use anyhow::Result;
use ironposh_client_core::connector::authenticator::{AuthenticaterStepResult, SspiAuthenticator};
use ironposh_client_core::connector::http::{HttpBuilder, HttpResponse, ServerAddress};
use sspi::{AuthIdentity, Secret, Username};

fn main() -> Result<()> {
    // Build your HTTP builder to the WinRM endpoint
    let http = HttpBuilder::new(
        ServerAddress::Ip(std::net::IpAddr::V4(std::net::Ipv4Addr::new(10, 10, 0, 3))),
        4453,
        ironposh_client_core::connector::Scheme::Http,
    );

    // Username/password for NTLM (DOMAIN\user or user@realm)
    let identity = AuthIdentity {
        username: Username::new_down_level_logon_name("administrator", "")?,
        password: Secret::new("DevoLabs123!".to_owned()),
    };

    // Create the authenticator (NTLM) and run step 1
    let mut auth: SspiAuthenticator<sspi::Ntlm> = SspiAuthenticator::new_ntlm(identity);
    // Authenticator::new_ntlm(Authentication::Sspi, http, Some(identity));

    match auth.step(None)? {
        AuthenticaterStepResult::SendBackAndContinue { token } => {}
        other => eprintln!("Unexpected: {:?}", other),
    }

    Ok(())
}
