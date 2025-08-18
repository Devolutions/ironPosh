use std::io::BufRead;
use std::net::Ipv4Addr;

use anyhow::Context;
use pwsh_core::connector::active_session::{PowershellOperations, UserEvent};
use pwsh_core::connector::{
    Authentication, Connector, ConnectorConfig, ConnectorStepResult, Scheme, SessionStepResult,
    UserOperation, http::ServerAddress,
};
use pwsh_core::powershell::PowerShell;
use tokio::io::AsyncBufReadExt;
use tokio::sync::{mpsc, oneshot};
use tracing::{Instrument, debug, error, info, info_span, warn};
use tracing_subscriber::EnvFilter;

pub enum NextStep {
    NetworkResponse(pwsh_core::connector::http::HttpResponse<String>),
    UserRequest(UserOperation),
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let log_file = std::fs::File::create("winrm_client.log")?;
    tracing_subscriber::fmt::SubscriberBuilder::default()
        .with_env_filter(EnvFilter::new("pwsh_core=debug,ureq=warn"))
        .with_max_level(tracing::Level::DEBUG)
        .with_target(false)
        .with_line_number(true)
        .with_file(true)
        .with_writer(log_file)
        .init();

    let _span = tracing::span!(tracing::Level::INFO, "main").entered();

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
    let (mut active_session, next_request) = {
        let mut response = None;

        let _span = info_span!("ConnectionLoop").entered();
        let (active_session, next_request) = loop {
            let step_result = connector
                .step(response.take())
                .context("Failed to step through connector")?;

            info!(step_result = ?step_result.name(), "Processing step result");

            match step_result {
                ConnectorStepResult::SendBack(http_request) => {
                    // Make the HTTP request (using ureq for simplicity in example)
                    response = Some(make_http_request(&http_request).await?);
                }
                ConnectorStepResult::SendBackError(e) => {
                    warn!("Connection step failed: {}", e);
                    anyhow::bail!("Connection failed: {}", e);
                }
                ConnectorStepResult::Connected {
                    active_session,
                    next_receive_request,
                } => {
                    break (active_session, next_receive_request);
                }
            }
        };
        drop(_span);
        (active_session, next_request)
    };

    info!("Runspace pool is now open and ready for operations!");

    let (network_request_tx, mut network_request_rx) = mpsc::channel(2);
    let (network_response_tx, mut network_response_rx) = mpsc::channel(2);

    tokio::spawn(
        async move {
            while let Some(request) = network_request_rx.recv().await {
                let network_response_tx = network_response_tx.clone();
                tokio::spawn(async move {
                    match make_http_request(&request).await {
                        Ok(response) => {
                            if let Err(e) = network_response_tx.send(response).await {
                                error!("Failed to send network response: {}", e);
                            }
                        }
                        Err(e) => {
                            error!("HTTP request failed: {}", e);
                        }
                    }
                });
            }
        }
        .instrument(info_span!("NetworkRequestHandler")),
    );

    let (user_request_tx, mut user_request_rx) = mpsc::channel(2);

    // Store the created pipeline for reuse
    let (pipeline_tx, pipeline_rx) = oneshot::channel();
    let user_request_tx_clone = user_request_tx.clone();

    tokio::spawn(
        async move {
            info!("Creating initial pipeline...");
            user_request_tx
                .send(UserOperation::CreatePipeline)
                .await
                .unwrap();

            // Wait for pipeline to be created
            let pipeline: PowerShell = pipeline_rx.await.unwrap();
            info!("Pipeline ready! Enter PowerShell commands (type 'exit' to quit):");

            let stdin = tokio::io::stdin();
            let mut reader = tokio::io::BufReader::new(stdin);
            let mut line = String::new();

            loop {
                print!("> ");
                std::io::Write::flush(&mut std::io::stdout()).unwrap();

                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) => break, // EOF
                    Ok(_) => {
                        let command = line.trim().to_string();
                        if command.to_lowercase() == "exit" {
                            info!("Exiting...");
                            break;
                        }
                        if !command.is_empty() {
                            // Add the script to the pipeline
                            user_request_tx_clone
                                .send(UserOperation::OperatePipeline {
                                    powershell: pipeline,
                                    operation: PowershellOperations::AddScript(command.clone()),
                                })
                                .await
                                .unwrap();

                            // Invoke the pipeline
                            user_request_tx_clone
                                .send(UserOperation::InvokePipeline {
                                    powershell: pipeline,
                                })
                                .await
                                .unwrap();
                        }
                    }
                    Err(e) => {
                        error!("Failed to read input: {}", e);
                        break;
                    }
                }
            }
        }
        .instrument(info_span!("UserInputHandler")),
    );

    network_request_tx
        .send(next_request)
        .await
        .context("Failed to send initial request")?;

    let mut pipeline_tx = Some(pipeline_tx);
    loop {
        let next_step = tokio::select! {
            network_response = network_response_rx.recv() => {
                if let Some(response) = network_response {
                    NextStep::NetworkResponse(response)
                } else {
                    error!("No response received from server");
                    return Err(anyhow::anyhow!("No response received from server"));
                }
            },
            user_request = user_request_rx.recv() => {
                if let Some(user_request) = user_request {
                    NextStep::UserRequest(user_request)
                } else {
                    error!("No user request received");
                    return Err(anyhow::anyhow!("No user request received"));
                }
            },
        };

        let step_result = match next_step {
            NextStep::NetworkResponse(http_response) => active_session
                .accept_server_response(http_response)
                .context("Failed to accept server response")?,
            NextStep::UserRequest(user_operation) => active_session
                .accept_client_operation(user_operation)
                .context("Failed to accept user operation")?,
        };

        info!("Received server response, processing...");

        match step_result {
            SessionStepResult::SendBack(http_requests) => {
                for http_request in http_requests {
                    network_request_tx
                        .send(http_request)
                        .await
                        .context("Failed to send HTTP request")?;
                }
            }
            SessionStepResult::SendBackError(e) => {
                error!("Error in session step: {}", e);
                return Err(anyhow::anyhow!("Session step failed: {}", e));
            }
            SessionStepResult::UserEvent(event) => match event {
                UserEvent::PipelineCreated { powershell } => {
                    info!("Pipeline created: {:?}", powershell);
                    let sent = pipeline_tx.take().map(|tx| tx.send(powershell));
                    if let Some(Err(_)) = sent {
                        error!("Failed to send pipeline through channel");
                        return Err(anyhow::anyhow!("Failed to send pipeline through channel"));
                    }
                }
            },
            SessionStepResult::OperationSuccess => {
                info!("Operation completed successfully");
            }
        }
    }
}

async fn make_http_request(
    request: &pwsh_core::connector::http::HttpRequest<String>,
) -> Result<pwsh_core::connector::http::HttpResponse<String>, anyhow::Error> {
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

    // Return as HttpResponse with proper response format
    Ok(pwsh_core::connector::http::HttpResponse {
        status_code: 200,
        headers: vec![],
        body: Some(response_body),
    })
}
