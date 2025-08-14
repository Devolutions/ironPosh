use std::net::Ipv4Addr;

use anyhow::Context;
use pwsh_core::connector::{http::ServerAddress, Authentication, Connector, ConnectorConfig, Scheme, ConnectorStepResult, SessionStepResult, UserOperation};
use tracing::{debug, info, warn};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
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
    let mut response = None;

    let mut active_session = loop {
        let step_result = connector
            .step(response.take())
            .context("Failed to step through connector")?;

        info!(step_result = ?step_result, "Processing step result");

        match step_result {
            ConnectorStepResult::SendBack(http_request) => {
                // Make the HTTP request (using ureq for simplicity in example)
                response = Some(make_http_request(&http_request).await?);
            }
            ConnectorStepResult::SendBackError(e) => {
                warn!("Connection step failed: {}", e);
                anyhow::bail!("Connection failed: {}", e);
            }
            ConnectorStepResult::Connected(session) => {
                info!("Successfully connected!");
                break session;
            }
        }
    };

    info!("Runspace pool is now open and ready for operations!");

    // Start the main operation loop
    let mut response = None;

    loop {
        // Check for any server responses first
        if let Some(server_response) = response.take() {
            let step_result = active_session.accept_server_response(server_response)?;
            match step_result {
                SessionStepResult::SendBack(http_request) => {
                    response = Some(make_http_request(&http_request).await.unwrap());
                }
                SessionStepResult::PipelineCreated(pipeline) => {
                    info!("Pipeline created successfully with ID: {}", pipeline.id());
                }
                SessionStepResult::SendBackError(e) => {
                    panic!("Server response error: {}", e);
                }
                _ => {
                    debug!("Received step result: {:?}", step_result);
                }
            }
        }

        // Prompt user for actions
        print!("Do you want to create a pipeline? (y/n): ");
        use std::io::{self, Write};
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        match input.trim().to_lowercase().as_str() {
            "y" | "yes" => {
                info!("Creating pipeline...");
                let step_result = active_session.accept_client_operation(UserOperation::CreatePipeline)?;
                match step_result {
                    SessionStepResult::SendBack(http_request) => {
                        response = Some(make_http_request(&http_request).await?);
                    }
                    _ => {
                        debug!("Unexpected step result from client operation: {:?}", step_result);
                    }
                }
            }
            "n" | "no" => {
                info!("Exiting...");
                break;
            }
            _ => {
                println!("Please enter 'y' for yes or 'n' for no.");
                continue;
            }
        }
    }

    info!("Exiting WinRM PowerShell client");
    Ok(())
}

async fn make_http_request(
    request: &pwsh_core::connector::http::HttpRequest<String>,
) -> Result<pwsh_core::connector::http::HttpRequest<String>, anyhow::Error> {
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
