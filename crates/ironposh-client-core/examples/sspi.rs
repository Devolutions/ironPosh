// examples/first_step.rs (or wherever you test)

use anyhow::Result;
use ironposh_client_core::ClientAuthIdentity;
use ironposh_client_core::connector::authenticator::{AuthenticatorStepResult, SspiAuthenticator};
use ironposh_client_core::connector::http::{HttpBuilder, HttpResponse, ServerAddress};
use ironposh_client_core::credentials::ClientUserName;

fn main() -> Result<()> {
    // // Build your HTTP builder to the WinRM endpoint
    // let http = HttpBuilder::new(
    //     ServerAddress::Ip(std::net::IpAddr::V4(std::net::Ipv4Addr::new(10, 10, 0, 3))),
    //     4453,
    //     ironposh_client_core::connector::Scheme::Http,
    // );

    // let username = ClientUserName::new("administrator", None)?;

    // // Username/password for NTLM (DOMAIN\user or user@realm)
    // let identity = ClientAuthIdentity::new(username, "DevoLabs123!".into());

    // // Create the authenticator (NTLM) and run step 1
    // let mut auth: SspiAuthenticator<sspi::Ntlm> = SspiAuthenticator::new_ntlm(identity);
    // // Authenticator::new_ntlm(Authentication::Sspi, http, Some(identity));

    // match auth.step(None)? {
    //     AuthenticatorStepResult::ContinueWithToken { token } => {}
    //     other => eprintln!("Unexpected: {:?}", other),
    // }

    Ok(())
}
