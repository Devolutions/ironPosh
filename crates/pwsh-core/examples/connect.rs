use std::net::Ipv4Addr;

use anyhow::Context;
use pwsh_core::connector::{
    Authentication, Connector, ConnectorConfig, ConnectorStepResult, Scheme, SessionStepResult,
    UserOperation, http::ServerAddress,
};
use tokio::sync::mpsc;
use tracing::{debug, error, info, info_span, warn, Instrument};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt::SubscriberBuilder::default()
        .with_env_filter(EnvFilter::new("pwsh_core=debug,ureq=warn"))
        .with_max_level(tracing::Level::DEBUG)
        .with_target(false)
        .with_line_number(true)
        .with_file(true)
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
    let mut response = None;

    let _span = info_span!("ConnectionLoop").entered();
    let active_session = loop {
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
                info!("Successfully connected!");
                response = Some(next_receive_request);
                break active_session;
            }
        }
    };
    drop(_span);

    info!(last_response = ?response,"Runspace pool is now open and ready for operations!");

    // Implement Actor Model for ActiveSession
    let (user_tx, user_rx) = mpsc::channel::<UserOperation>(32);
    let (server_tx, server_rx) =
        mpsc::channel::<pwsh_core::connector::http::HttpRequest<String>>(32);
    let (network_tx, network_rx) =
        mpsc::channel::<pwsh_core::connector::http::HttpRequest<String>>(32);
    let (ui_tx, ui_rx) = mpsc::channel::<String>(32);

    // Spawn the ActiveSession Actor Task
    let active_actor_span = info_span!("ActiveSessionActor");
    let actor_handle = tokio::spawn(async move {
        let mut session = active_session;
        let mut user_rx = user_rx;
        let mut server_rx = server_rx;
        let network_tx = network_tx;
        let ui_tx = ui_tx;

        info!("ActiveSession actor started");

        loop {
            tokio::select! {
                Some(user_operation) = user_rx.recv() => {
                    info!("Actor received user operation: {:?}", user_operation);
                    match session.accept_client_operation(user_operation) {
                        Ok(result) => {
                            match result {
                                SessionStepResult::SendBack(http_request) => {
                                    info!("Actor sending HTTP request to network task");
                                    if let Err(e) = network_tx.send(http_request).await {
                                        warn!("Failed to send HTTP request to network task: {}", e);
                                        break;
                                    }
                                }
                                SessionStepResult::PipelineCreated(pipeline) => {
                                    let msg = format!("Pipeline created successfully! ID: {}", pipeline.id());
                                    info!("{}", msg);
                                    let _ = ui_tx.send(msg).await;
                                }
                                SessionStepResult::SendBackError(e) => {
                                    let msg = format!("Error in user operation: {}", e);
                                    warn!("{}", msg);
                                    let _ = ui_tx.send(msg).await;
                                }
                                _ => {
                                    debug!("Unexpected result from user operation: {:?}", result);
                                }
                            }
                        }
                        Err(e) => {
                            let msg = format!("Failed to process user operation: {}", e);
                            warn!("{}", msg);
                            let _ = ui_tx.send(msg).await;
                        }
                    }
                }
                Some(server_response) = server_rx.recv() => {
                    info!("Actor received server response");
                    match session.accept_server_response(server_response) {
                        Ok(result) => {
                            match result {
                                SessionStepResult::SendBack(http_request) => {
                                    info!("Actor sending follow-up HTTP request to network task");
                                    if let Err(e) = network_tx.send(http_request).await {
                                        warn!("Failed to send HTTP request to network task: {}", e);
                                        break;
                                    }
                                }
                                SessionStepResult::PipelineCreated(pipeline) => {
                                    let msg = format!("Pipeline created from server response! ID: {}", pipeline.id());
                                    info!("{}", msg);
                                    let _ = ui_tx.send(msg).await;
                                }
                                SessionStepResult::SendBackError(e) => {
                                    let msg = format!("Error in server response: {}", e);
                                    warn!("{}", msg);
                                    let _ = ui_tx.send(msg).await;
                                }
                                _ => {
                                    debug!("Unexpected result from server response: {:?}", result);
                                }
                            }
                        }
                        Err(e) => {
                            let msg = format!("Failed to process server response: {}", e);
                            error!("{}", msg);
                            let _ = ui_tx.send(msg).await;
                        }
                    }
                }
            }
        }

        info!("ActiveSession actor shutting down");
    }.instrument(active_actor_span));

    // Spawn the Network Task
    let network_actor_span = info_span!("NetworkTask");
    let network_handle = tokio::spawn(
        async move {
            let mut network_rx = network_rx;
            let server_tx = server_tx;

            info!("Network task started");
            if let Some(response) = response.take() {
                let _ = server_tx
                    .send(response)
                    .await
                    .inspect_err(|e| warn!("Failed to send initial response back to actor: {}", e));
            }

            while let Some(http_request) = network_rx.recv().await {
                info!("Network task making HTTP request");
                match make_http_request(&http_request).await {
                    Ok(response) => {
                        if let Err(e) = server_tx.send(response).await {
                            warn!("Failed to send response back to actor: {}", e);
                            break;
                        }
                    }
                    Err(e) => {
                        warn!("HTTP request failed: {}", e);
                        // Continue with the next request
                    }
                }
            }

            info!("Network task shutting down");
        }
        .instrument(network_actor_span),
    );

    // Spawn the User Input Task (Auto-create pipeline after delay)
    let user_actor_span = info_span!("UserInputTask");
    let user_input_handle = tokio::spawn(
        async move {
            let user_tx = user_tx;

            info!("User input task started - will auto-create pipeline after 3 seconds");

            // Wait 3 seconds to let the connection stabilize
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

            info!("Auto-creating pipeline...");
            if let Err(e) = user_tx.send(UserOperation::CreatePipeline).await {
                warn!("Failed to send user operation to actor: {}", e);
                return;
            }

            // Wait another 5 seconds to let the pipeline work
            tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

            info!("Auto-exiting after pipeline execution");

            info!("User input task shutting down");
        }
        .instrument(user_actor_span),
    );

    // Main task handles UI feedback
    let ui_handle = tokio::spawn(async move {
        let mut ui_rx = ui_rx;

        info!("UI feedback task started");

        while let Some(message) = ui_rx.recv().await {
            println!("📢 {}", message);
        }

        info!("UI feedback task shutting down");
    });

    // Wait for user input task to complete (user chooses to exit)
    let _ = user_input_handle.await;

    // Wait for other tasks to complete
    let _ = tokio::join!(actor_handle, network_handle, ui_handle);

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
