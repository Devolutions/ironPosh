use anyhow::Result;
use ironposh_client_core::{
    KerberosConfig, SspiAuthConfig,
    connector::{
        auth_sequence::{AuthSequence, SecurityContextBuilderHolder},
        authenticator::{ActionReqired, SecContextMaybeInit},
        http::{HttpBuilder, ServerAddress},
    },
    credentials::{ClientAuthIdentity, ClientUserName},
};
use url::Url;

fn main() -> Result<()> {
    let username = ClientUserName::new("administrator", None)?;
    let identity = ClientAuthIdentity::new(username, "DevoLabs123!".into());
    let sspi_config = SspiAuthConfig::Kerberos {
        target_name: "HTTP/IT-HELP.DC.ad.it-help.ninja".into(),
        identity,
        kerberos_config: KerberosConfig {
            kdc_url: Some(Url::parse("tcp://IT-HELP.DC.ad.it-help.ninja:88")?),
            client_computer_name: None,
        },
    };
    let http_builder = HttpBuilder::new(
        ServerAddress::Domain("www.example.com".into()),
        80,
        ironposh_client_core::connector::Scheme::Http,
    );

    let mut auth_sequence = AuthSequence::new(sspi_config, http_builder)?;

    let final_token = loop {
        let mut holder = SecurityContextBuilderHolder::new();
        let init = {
            let result = auth_sequence.try_init_sec_context(None, &mut holder)?;
            let init = match result {
                SecContextMaybeInit::Initialized(sec_context_init) => sec_context_init,
                SecContextMaybeInit::RunGenerator {
                    mut packet,
                    mut generator_holder,
                } => loop {
                    let response = send(packet)?;
                    match AuthSequence::resume(generator_holder, response)? {
                        SecContextMaybeInit::Initialized(sec_context_init) => {
                            break sec_context_init;
                        }
                        SecContextMaybeInit::RunGenerator {
                            packet: packet2,
                            generator_holder: generator2,
                        } => {
                            packet = packet2;
                            generator_holder = generator2;
                        }
                    }
                },
            };
            init
        };

        println!("Would process context here and potentially loop again");

        drop(holder); // Need to explicitly drop the holder to release borrows
        let action = auth_sequence.process_initialized_sec_context(init)?;

        match action {
            ActionReqired::TryInitSecContextAgain { token } => {
                // In a real scenario, you would use this token in the next HTTP request.
                println!("Got token for next round: {:?}", token);
                todo!("Send HTTP request with token and get response");
            }
            ActionReqired::Done { token } => {
                println!("Authentication finished. Final token: {:?}", token);
                break token;
            }
        }
    };

    println!("Final token: {:?}", final_token);

    Ok(())
}

pub fn send(packet: sspi::generator::NetworkRequest) -> Result<Vec<u8>> {
    println!("Sending packet to server...{:?}", packet);
    panic!("Not implemented");
    // In a real implementation, this would send the packet over the network
    // and return the server's response.
    // For this example, we return an empty Vec<u8> to simulate a response.
    Ok(Vec::new())
}
