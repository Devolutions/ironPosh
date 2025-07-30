use pwsh_core::connector::{ConnectorConfig, ConnectorResult, Credentials, Target};

fn main() {
    tracing_subscriber::fmt::SubscriberBuilder::default()
        .with_target(false)
        .with_line_number(true)
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let host = "10.10.0.3";

    let mut connector = pwsh_core::connector::Connector::new(
        ConnectorConfig::builder()
            .target(Target {
                host: pwsh_core::connector::Host::IpAddress(host.parse().unwrap()),
                scheme: pwsh_core::connector::HttpSchema::Http,
            })
            .auth(pwsh_core::connector::Auth::Basic(Credentials {
                username: "administrator".to_string(),
                password: "DevoLabs123!".to_string(),
            }))
            .build(),
    );

    let output = connector.step(None).expect("Failed to connect");

    let ConnectorResult { message, headers } = output;

    println!("Message: {}", message);
    println!("Headers: {:?}", headers);

    let mut request = ureq::post("http://10.10.0.3:5985/wsman?PSVersion=7.4.10");

    for (header, value) in headers {
        request = request.set(&header, &value);
    }

    let response = request
        .send_string(&message)
        .expect("Failed to send request");

    println!("Response: {:?}", response)
}
