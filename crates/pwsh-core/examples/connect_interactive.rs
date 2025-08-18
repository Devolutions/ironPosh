
/// THIS EXAMPLE IS NOT COMPLETE YET!


use std::collections::HashMap;
use std::net::Ipv4Addr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use anyhow::Context;
use pwsh_core::connector::active_session::{PowershellOperations, UserEvent};
use pwsh_core::connector::{
    Authentication, Connector, ConnectorConfig, ConnectorStepResult, Scheme, SessionStepResult,
    UserOperation, http::ServerAddress,
};
use pwsh_core::powershell::PowerShell;
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
    let (user_request_tx, user_request_rx) = mpsc::channel::<(
        UserOperation,
        oneshot::Sender<anyhow::Result<SessionStepResult>>,
    )>(32);
    let (session_cmd_tx, session_cmd_rx) = mpsc::channel::<SessionCommand>(32);
    let (network_request_tx, network_request_rx) = mpsc::channel::<NetworkRequest>(32);
    let (network_response_tx, network_response_rx) =
        mpsc::channel::<pwsh_core::connector::http::HttpResponse<String>>(32);
    let (ui_tx, ui_rx) = mpsc::channel(32);

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
                                SessionStepResult::UserEvent(user_event) => {
                                    info!("SessionManager emitting user event: {:?}", user_event);
                                    let _ = response_tx.send(Ok(SessionStepResult::UserEvent(user_event)));
                                }
                                other_result => {
                                    info!("SessionManager immediate result: {:?}", other_result);
                                    
                                    // Send UI feedback for immediate results
                                    match &other_result {
                                        SessionStepResult::SendBackError(e) => {
                                            error!(?e, "Error in user operation");
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
                                SessionStepResult::UserEvent(user_event) => {
                                    info!(user_event = ?user_event, "SessionManager emitting user event");
                                    let _ = ui_tx.send(user_event).await;
                                }
                                SessionStepResult::SendBackError(e) => {
                                    error!(?e, "Error in server response processing");
                                }
                                _ => {
                                    debug!("Server response result: {:?}", result);
                                }
                            }
                        }
                        Err(e) => {
                            error!(?e, "Failed to process server response");
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
                    .send(SessionCommand::ProcessServerResponse {
                        response: initial_response,
                    })
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
                            session_cmd_tx
                                .send(SessionCommand::ProcessServerResponse { response })
                                .await?;
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
                        session_cmd_tx
                            .send(SessionCommand::ProcessServerResponse { response })
                            .await?;
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
                info!(
                    "RequestDispatcher processing operation: {:?} (correlation: {})",
                    operation, correlation_id
                );

                let session_cmd = SessionCommand::ProcessUserOperation {
                    correlation_id,
                    operation,
                    response_tx,
                };

                // Non-blocking send to session manager
                session_cmd_tx.try_send(session_cmd).map_err(|e| {
                    anyhow::anyhow!("Failed to send user operation to session manager: {}", e)
                })?;
            }

            info!("RequestDispatcher shutting down");
            anyhow::Ok(())
        }
        .instrument(request_dispatcher_span),
    );

    let user_context = Arc::new(Mutex::new(HashMap::<String,Vec<PowerShell>>::new()));
    let user_actor_span = info_span!("UserInputTask");
    let user_context_clone = Arc::clone(&user_context);
    let user_input_handle = tokio::spawn(
        async move {
            let user_request_tx = user_request_tx;

            info!("User input task started");

            let welcome_msg = "Welcome to the WinRM PowerShell client!";
            let menu = r#"Choose your operation 
                1. Create Pipeline
                2. Create a Command to Pipeline
                3. Invoke Pipeline
                4. Exit
            "#;

            println!("{}", welcome_msg);

            let mut selected_powershell: Option<PowerShell> = None;

            loop {
                print!("{}\n> ", menu);
                use std::io::Write;
                std::io::stdout().flush().unwrap();
                let mut input = String::new();
                std::io::stdin().read_line(&mut input).unwrap();
                let input = input.trim();

                let operation = match input {
                    "1" => {
                        UserOperation::CreatePipeline
                    }
                    "2"   => {
                        /*
                          Expected behavior for these operations
                          ```
                            Choose a power shell to operate on
                                1. PowerShell GUID: <GUID>
                                ...
                            then ask for command like this:
                            Enter command to add to pipeline, enter to finish:
                            > Write-Host "Remote System: $($env:COMPUTERNAME) - $(Get-Date)"
                          ```
                        */

                        let user_context_map = user_context_clone.lock().unwrap();

                        let Some(powershell) = user_context_map.get("PowerShell") else {
                            println!("No PowerShell pipeline created yet. Please create one first.");
                            continue;
                        };

                        println!("Choose a PowerShell pipeline to operate on:");

                        for (i, ps) in powershell.iter().enumerate() {
                            println!("{}: PowerShell GUID: {}", i + 1, ps.id());
                        }

                        print!("Enter the number of the PowerShell pipeline to operate on: ");
                        std::io::stdout().flush().unwrap();
                        let mut choice = String::new();
                        std::io::stdin().read_line(&mut choice).unwrap();
                        let choice: usize = match choice.trim().parse() {
                            Ok(num) if num > 0 && num <= powershell.len() => num - 1,
                            _ => {
                                println!("Invalid choice, please try again.");
                                continue;
                            }
                        };

                        selected_powershell = Some(powershell[choice].clone());
                        println!("Selected PowerShell pipeline: {}", selected_powershell.unwrap().id());

                        print!("Enter command to add to pipeline, press enter to finish: ");
                        std::io::stdout().flush().unwrap();
                        let mut command = String::new();
                        std::io::stdin().read_line(&mut command).unwrap();
                        let command = command.trim().to_string();   
                        if command.is_empty() {
                            println!("No command entered, skipping operation.");
                            continue;
                        }

                        let operation = UserOperation::OperatePipeline {
                            powershell: selected_powershell.unwrap().clone(),
                            operation: PowershellOperations::AddCommand(command),
                        };

                        operation
                    }
                    "3" => {
                        if let Some(powershell) = &selected_powershell {
                            UserOperation::InvokePipeline {
                                powershell: powershell.clone(),
                            }
                        } else {
                            println!("No PowerShell pipeline selected. Please create or select one first.");
                            continue;
                        }
                    }
                    "4" => {
                        println!("Exiting...");
                        break;
                    }
                    _ => {
                        println!("Invalid choice, please try again.");
                        continue;
                    }
                };


                todo!()
                // session_cmd_tx.send(SessionCommand::ProcessUserOperation { correlation_id: (), operation: (), response_tx: () })
            };


            anyhow::Ok(())
        }
        .instrument(user_actor_span),
    );

    // Main task handles UI feedback
    let ui_handle = tokio::spawn(async move {
        let mut ui_rx = ui_rx;

        info!("UI feedback task started");

        while let Some(message) = ui_rx.recv().await {
            println!("** UI Message: {:?}", message);
            match message {
                UserEvent::PipelineCreated { powershell } => {
                    println!("Pipeline created: {:?}", powershell);
                    user_context.lock().unwrap().insert(
                        "PowerShell".to_string(),
                        vec![powershell],
                    );
                }
                _ => {
                    println!("Received user event: {:?}", message);
                }
                
            }
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
