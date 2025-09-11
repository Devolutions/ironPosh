use anyhow::Result;
use ironposh_client_core::ClientAuthIdentity;
use ironposh_client_core::connector::authenticator::{
    SspiConext, SecContextInit, SecContextMaybeInit, SspiAuthenticator,
};
use ironposh_client_core::credentials::ClientUserName;
use sspi::Sspi;

fn main() -> Result<()> {
    let username = ClientUserName::new("administrator", None)?;
    let identity = ClientAuthIdentity::new(username, "DevoLabs123!".into());

    // Create NTLM context
    let mut context = SspiConext::new_ntlm(
        identity,
        ironposh_client_core::SspiAuthConfig::Negotiate {
            target: (),
            identity: (),
            kerberos_config: (),
        },
    )?;

    // let mut auth_response = None;

    // THIS IS THE PROBLEMATIC LOOP PATTERN FROM connection.rs
    let mut holder = None;
    let final_token = loop {
        // Let's try wrapping EVERYTHING in a block
        let init = {
            let result = SspiAuthenticator::try_init_sec_context(None, &mut context, &mut holder)?;
            let init = match result {
                SecContextMaybeInit::Initialized(sec_context_init) => sec_context_init,
                SecContextMaybeInit::RunGenerator {
                    mut packet,
                    mut generator_holder,
                } => {
                    loop {
                        let response = send(packet)?;
                        match SspiAuthenticator::resume(generator_holder, response)? {
                            SecContextMaybeInit::Initialized(sec_context_init) => {
                                break sec_context_init;
                            }
                            SecContextMaybeInit::RunGenerator {
                                packet: packet2,
                                generator_holder: generator2,
                            } => {
                                // Continue the loop with new packet and generator
                                packet = packet2;
                                generator_holder = generator2;
                            }
                        }
                    }
                }
            };

            init
        }; // The entire borrow scope ends here

        println!("Would process context here and potentially loop again");

        holder = None; // Clear the holder to simulate breaking the loop
        let action = SspiAuthenticator::process_initialized_sec_context(&mut context, init)?;

        match action {
            ironposh_client_core::connector::authenticator::ActionReqired::TryInitSecContextAgain { token } => {
                continue;

            },
            ironposh_client_core::connector::authenticator::ActionReqired::Done { token } => {
                break token;
            },
        }
    };

    Ok(())
}

pub fn send(packet: sspi::generator::NetworkRequest) -> Result<Vec<u8>> {
    // send the packet over the network
    Ok(Vec::new())
}
