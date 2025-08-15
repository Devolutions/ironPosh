use std::net::Ipv4Addr;
use std::collections::HashMap;
use std::time::Duration;

use anyhow::Context;
use pwsh_core::connector::{
    Authentication, Connector, ConnectorConfig, ConnectorStepResult, Scheme, SessionStepResult,
    UserOperation, http::ServerAddress,
};
use tokio::sync::{mpsc, oneshot};
use tokio::time::timeout;
use tracing::{Instrument, debug, error, info, info_span, warn};
use tracing_subscriber::EnvFilter;
use uuid::Uuid;

// Data structures for the new concurrent architecture
#[derive(Debug)]
struct PendingRequest {
    correlation_id: Uuid,
    operation: UserOperation,
    response_tx: oneshot::Sender<anyhow::Result<SessionStepResult>>,
}

#[derive(Debug)]
enum SessionCommand {
    ProcessUserOperation {
        correlation_id: Uuid,
        operation: UserOperation,
        response_tx: oneshot::Sender<anyhow::Result<SessionStepResult>>,
    },
    ProcessServerResponse {
        response: pwsh_core::connector::http::HttpResponse<String>,
    },
}

#[derive(Debug)]
enum NetworkRequest {
    HttpRequest(pwsh_core::connector::http::HttpRequest<String>),
    ContinuousReceive,
}

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
    let (active_session, next_request) = {
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

    // Setup channels for the new concurrent architecture
    let (user_request_tx, user_request_rx) = mpsc::channel::<(UserOperation, oneshot::Sender<anyhow::Result<SessionStepResult>>)>(32);
    let (session_cmd_tx, session_cmd_rx) = mpsc::channel::<SessionCommand>(32);
    let (network_request_tx, network_request_rx) = mpsc::channel::<NetworkRequest>(32);
    let (network_response_tx, network_response_rx) = mpsc::channel::<pwsh_core::connector::http::HttpResponse<String>>(32);
    let (ui_tx, ui_rx) = mpsc::channel::<String>(32);
    
    // Clone senders for multiple consumers
    let session_cmd_tx_for_network = session_cmd_tx.clone();
    let session_cmd_tx_for_receive = session_cmd_tx.clone();
    let session_cmd_tx_for_dispatcher = session_cmd_tx.clone();

    // Spawn the SessionManager Actor - manages session state and correlates requests
    let session_manager_span = info_span!("SessionManager");
    let session_handle = tokio::spawn(async move {
        let mut session = active_session;
        let mut pending_requests: HashMap<Uuid, PendingRequest> = HashMap::new();
        let mut session_cmd_rx = session_cmd_rx;
        let network_request_tx = network_request_tx;
        let ui_tx = ui_tx;

        info!("SessionManager started");

        while let Some(cmd) = session_cmd_rx.recv().await {
            match cmd {
                SessionCommand::ProcessUserOperation { correlation_id, operation, response_tx } => {
                    info!("SessionManager processing user operation: {:?} (correlation: {})", operation, correlation_id);
                    
                    match session.accept_client_operation(operation) {
                        Ok(result) => {
                            match result {
                                SessionStepResult::SendBack(http_request) => {
                                    info!("SessionManager queuing HTTP request for correlation: {}", correlation_id);
                                    pending_requests.insert(correlation_id, PendingRequest {
                                        correlation_id,
                                        operation: UserOperation::CreatePipeline, // Use a dummy operation since we already consumed the original
                                        response_tx,
                                    });
                                    
                                    if let Err(e) = network_request_tx.send(NetworkRequest::HttpRequest(http_request)).await {
                                        warn!("Failed to send HTTP request to network: {}", e);
                                        if let Some(pending) = pending_requests.remove(&correlation_id) {
                                            let _ = pending.response_tx.send(Err(anyhow::anyhow!("Network send failed: {}", e)));
                                        }
                                    }
                                }
                                other_result => {
                                    info!("SessionManager immediate result: {:?}", other_result);
                                    
                                    // Send UI feedback for immediate results
                                    match &other_result {
                                        SessionStepResult::PipelineCreated(pipeline) => {
                                            let msg = format!("Pipeline created successfully! ID: {}", pipeline.id());
                                            let _ = ui_tx.send(msg).await;
                                        }
                                        SessionStepResult::SendBackError(e) => {
                                            let msg = format!("Error in user operation: {}", e);
                                            let _ = ui_tx.send(msg).await;
                                        }
                                        _ => {}
                                    }
                                    
                                    let _ = response_tx.send(Ok(other_result));
                                }
                            }
                        }
                        Err(e) => {
                            warn!("Failed to process user operation: {}", e);
                            let _ = response_tx.send(Err(anyhow::anyhow!("Session error: {}", e)));
                        }
                    }
                }
                SessionCommand::ProcessServerResponse { response } => {
                    info!("SessionManager processing server response");
                    match session.accept_server_response(response) {
                        Ok(result) => {
                            match result {
                                SessionStepResult::SendBack(http_request) => {
                                    info!("SessionManager sending follow-up HTTP request");
                                    let _ = network_request_tx.send(NetworkRequest::HttpRequest(http_request)).await;
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
                                    debug!("Server response result: {:?}", result);
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to process server response: {}", e);
                            let msg = format!("Failed to process server response: {}", e);
                            let _ = ui_tx.send(msg).await;
                        }
                    }
                    
                    // Complete any pending requests that might be resolved by this server response
                    // For now, we'll complete the first pending request as a simple approach
                    if let Some((_, pending)) = pending_requests.drain().next() {
                        let _ = pending.response_tx.send(Ok(SessionStepResult::SendBackError(pwsh_core::PwshCoreError::ConnectorError("Completed by server response".to_string()))));
                    }
                }
            }
        }

        info!("SessionManager shutting down");
        anyhow::Ok(())
    }.instrument(session_manager_span));

    // Spawn the Network Pool - handles HTTP requests concurrently
    let network_pool_span = info_span!("NetworkPool");
    let network_handle = tokio::spawn(
        async move {
            let mut network_request_rx = network_request_rx;
            let session_cmd_tx = session_cmd_tx_for_network;

            info!("NetworkPool started");
            
            // Spawn initial response handler separately to avoid blocking the pool
            let initial_session_tx = session_cmd_tx.clone();
            tokio::spawn(async move {
                let initial_response = make_http_request(&next_request).await?;
                info!("NetworkPool received initial response");
                initial_session_tx
                    .send(SessionCommand::ProcessServerResponse { response: initial_response })
                    .await?;
                anyhow::Ok(())
            });

            while let Some(network_request) = network_request_rx.recv().await {
                match network_request {
                    NetworkRequest::HttpRequest(http_request) => {
                        info!("NetworkPool making user-requested HTTP request");
                        let session_cmd_tx = session_cmd_tx.clone();
                        
                        // Spawn individual request handlers for concurrency
                        tokio::spawn(async move {
                            let response = make_http_request(&http_request).await?;
                            session_cmd_tx.send(SessionCommand::ProcessServerResponse { response }).await?;
                            anyhow::Ok(())
                        });
                    }
                    NetworkRequest::ContinuousReceive => {
                        info!("NetworkPool handling continuous receive request");
                        // This would be implemented based on the specific receive mechanism
                        // For now, we'll implement it as a placeholder
                    }
                }
            }

            info!("NetworkPool shutting down");
            anyhow::Ok(())
        }
        .instrument(network_pool_span),
    );

    // Spawn the ReceiveActor - continuous long-polling for server messages
    let receive_actor_span = info_span!("ReceiveActor");
    let receive_handle = tokio::spawn(
        async move {
            let session_cmd_tx = session_cmd_tx_for_receive;
            let _network_response_tx = network_response_tx;
            let mut network_response_rx = network_response_rx;

            info!("ReceiveActor started - beginning continuous receive loop");

            loop {
                // Create a receive request (this would be protocol-specific)
                // For now, we'll simulate continuous receiving with a timeout
                match timeout(Duration::from_secs(30), network_response_rx.recv()).await {
                    Ok(Some(response)) => {
                        info!("ReceiveActor received server message");
                        session_cmd_tx.send(SessionCommand::ProcessServerResponse { response }).await?;
                    }
                    Ok(None) => {
                        info!("ReceiveActor channel closed");
                        break;
                    }
                    Err(_) => {
                        // Timeout - this is normal for long-polling, continue the loop
                        debug!("ReceiveActor timeout - retrying receive");
                        continue;
                    }
                }
            }
            
            info!("ReceiveActor shutting down");
            anyhow::Ok(())
        }
        .instrument(receive_actor_span),
    );

    // Spawn the RequestDispatcher - handles concurrent user operations
    let request_dispatcher_span = info_span!("RequestDispatcher");
    let request_dispatcher_handle = tokio::spawn(
        async move {
            let mut user_request_rx = user_request_rx;
            let session_cmd_tx = session_cmd_tx_for_dispatcher;

            info!("RequestDispatcher started");

            while let Some((operation, response_tx)) = user_request_rx.recv().await {
                let correlation_id = Uuid::new_v4();
                info!("RequestDispatcher processing operation: {:?} (correlation: {})", operation, correlation_id);

                let session_cmd = SessionCommand::ProcessUserOperation {
                    correlation_id,
                    operation,
                    response_tx,
                };

                // Non-blocking send to session manager
                session_cmd_tx.try_send(session_cmd)
                    .map_err(|e| anyhow::anyhow!("Failed to send user operation to session manager: {}", e))?;
            }

            info!("RequestDispatcher shutting down");
            anyhow::Ok(())
        }
        .instrument(request_dispatcher_span),
    );

    // Spawn the User Input Task (Auto-create pipeline after delay) - now uses the new architecture
    let user_actor_span = info_span!("UserInputTask");
    let user_input_handle = tokio::spawn(
        async move {
            let user_request_tx = user_request_tx;

            info!("User input task started - will auto-create pipeline after 3 seconds");

            // Wait 3 seconds to let the connection stabilize
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

            info!("Auto-creating first pipeline...");
            let (response_tx, response_rx) = oneshot::channel();
            user_request_tx.send((UserOperation::CreatePipeline, response_tx)).await?;

            // Wait for response
            let result = response_rx.await??;
            info!("First pipeline request completed: {:?}", result);

            // Wait 2 seconds, then create a second pipeline to demonstrate concurrency
            tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

            info!("Auto-creating second pipeline concurrently...");
            let (response_tx2, response_rx2) = oneshot::channel();
            user_request_tx.send((UserOperation::CreatePipeline, response_tx2)).await?;

            // Wait for second response
            let result2 = response_rx2.await??;
            info!("Second pipeline request completed: {:?}", result2);

            // Wait another 3 seconds to let everything settle
            tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;

            info!("Auto-exiting after concurrent pipeline execution");
            info!("User input task shutting down");
            anyhow::Ok(())
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
        anyhow::Ok(())
    });

    // Wait for user input task to complete (user chooses to exit)
    let _ = user_input_handle.await;

    // Wait for other tasks to complete
    let _ = tokio::join!(
        session_handle,
        network_handle, 
        receive_handle,
        request_dispatcher_handle,
        ui_handle
    );

    info!("Exiting WinRM PowerShell client");
    Ok(())
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
