use std::net::Ipv4Addr;

use pwsh_core::{
    connector::http::ServerAddress,
    connector::{Authentication, Connector, ConnectorConfig, Scheme, StepResult},
};
use tracing::{debug, info, warn};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::SubscriberBuilder::default()
        .with_target(false)
        .with_line_number(true)
        .with_file(true)
        .with_max_level(tracing::Level::DEBUG)
        .init();

    info!("Starting WinRM PowerShell client");

    // Configuration - modify these for your test server
    let server = ServerAddress::Ip(std::net::IpAddr::V4(Ipv4Addr::new(10, 10, 0, 3))); // Change to your server
    let port = 5985;
    let scheme = Scheme::Http;
    let auth = Authentication::Basic {
        username: "Administrator".to_string(),
        password: "DevoLabs123!".to_string(),
    };

    let config = ConnectorConfig {
        server: (server, port),
        scheme,
        authentication: auth,
    };

    let mut connector = Connector::new(config);

    info!("Created connector, starting connection...");

    // Step 1: Initial connection (should return shell create request)
    let step_result = connector.step(None)?;

    match step_result {
        StepResult::SendBack(http_request) => {
            debug!("Got initial request: {:?}", http_request);

            // Make the HTTP request (using ureq for simplicity in example)
            let response = make_http_request(&http_request).await?;

            // Step 2: Process shell create response
            let step_result = connector.step(Some(response))?;

            match step_result {
                StepResult::SendBack(receive_request) => {
                    debug!("Got receive request: {:?}", receive_request);

                    // Make the receive request
                    let receive_response = make_http_request(&receive_request).await?;

                    // Step 3: Process receive response (should hit the todo!() for now)
                    let step_result = connector.step(Some(receive_response));

                    match step_result {
                        Ok(StepResult::ReadyForOperation) => {
                            info!("Connection established successfully!");
                        }
                        Ok(other) => {
                            info!("Got step result: {:?}", other);
                        }
                        Err(e) => {
                            warn!("Step failed (expected due to todo!()): {}", e);
                        }
                    }
                }
                StepResult::SendBackError(e) => {
                    warn!("Connection failed: {}", e);
                }
                other => {
                    info!("Unexpected step result: {:?}", other);
                }
            }
        }
        StepResult::SendBackError(e) => {
            warn!("Initial step failed: {}", e);
        }
        other => {
            info!("Unexpected initial step result: {:?}", other);
        }
    }

    Ok(())
}

async fn make_http_request(
    request: &pwsh_core::connector::http::HttpRequest<String>,
) -> Result<pwsh_core::connector::http::HttpRequest<String>, Box<dyn std::error::Error>> {
    info!("Making HTTP request to: {}", request.url);
    debug!("Request headers: {:?}", request.headers);
    debug!(
        "Request body length: {:?}",
        request.body.as_ref().map(|b| b.len())
    );

    // Build the HTTP client request
    let mut ureq_request = match request.method {
        pwsh_core::connector::http::Method::Post => ureq::post(&request.url),
        pwsh_core::connector::http::Method::Get => ureq::get(&request.url),
        pwsh_core::connector::http::Method::Put => ureq::put(&request.url),
        pwsh_core::connector::http::Method::Delete => ureq::delete(&request.url),
    };

    // Add headers
    for (name, value) in &request.headers {
        ureq_request = ureq_request.set(name, value);
    }

    // Add cookie if present
    if let Some(cookie) = &request.cookie {
        ureq_request = ureq_request.set("Cookie", cookie);
    }

    // Make the request
    let response = if let Some(body) = &request.body {
        ureq_request.send_string(body)?
    } else {
        ureq_request.call()?
    };

    // Read response
    let response_body = response.into_string()?;
    debug!("Response body length: {}", response_body.len());

    // Return as HttpRequest (simulating the response format)
    Ok(pwsh_core::connector::http::HttpRequest {
        method: pwsh_core::connector::http::Method::Post,
        url: request.url.clone(),
        headers: vec![],
        body: Some(response_body),
        cookie: None,
    })
}
