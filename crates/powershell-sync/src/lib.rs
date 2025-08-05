use pwsh_core::{
    connector::{http::HttpRequest, http::Method, Connector, ConnectorConfig, StepResult},
    PwshCoreError,
};
use reqwest::blocking::Client;
use std::collections::HashMap;
use thiserror::Error;
use tracing::{debug, error, info};

#[derive(Debug, Error)]
pub enum PowerShellSyncError {
    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),
    
    #[error("Core error: {0}")]
    CoreError(#[from] PwshCoreError),
    
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
}

pub struct PowerShellSyncClient {
    connector: Connector,
    http_client: Client,
}

impl PowerShellSyncClient {
    pub fn new(config: ConnectorConfig) -> Self {
        Self {
            connector: Connector::new(config),
            http_client: Client::new(),
        }
    }
    
    pub fn connect(&mut self) -> Result<(), PowerShellSyncError> {
        info!("Starting PowerShell connection");
        
        // Start the connection process
        let step_result = self.connector.step(None)?;
        
        match step_result {
            StepResult::SendBack(request) => {
                info!("Sending initial shell creation request");
                let response = self.execute_http_request(request)?;
                
                // Process the response
                let step_result = self.connector.step(Some(HttpRequest {
                    method: Method::Post,
                    url: String::new(), // Not used in response processing
                    headers: vec![],
                    body: Some(response),
                    cookie: None,
                }))?;
                
                match step_result {
                    StepResult::ReadyForOperation => {
                        info!("PowerShell connection established successfully");
                        Ok(())
                    }
                    StepResult::SendBack(_) => {
                        error!("Unexpected additional request needed");
                        Err(PowerShellSyncError::InvalidResponse(
                            "Expected ready state but got another request".to_string()
                        ))
                    }
                    StepResult::SendBackError(err) => {
                        error!("Connector returned error: {:?}", err);
                        Err(PowerShellSyncError::CoreError(err))
                    }
                    StepResult::UserEvent(_) => {
                        error!("Unexpected user event");
                        Err(PowerShellSyncError::InvalidResponse(
                            "Unexpected user event during connection".to_string()
                        ))
                    }
                }
            }
            StepResult::SendBackError(err) => {
                error!("Initial connection step failed: {:?}", err);
                Err(PowerShellSyncError::CoreError(err))
            }
            _ => {
                error!("Unexpected step result: {:?}", step_result);
                Err(PowerShellSyncError::InvalidResponse(
                    "Unexpected initial step result".to_string()
                ))
            }
        }
    }
    
    fn execute_http_request(&self, request: HttpRequest<String>) -> Result<String, PowerShellSyncError> {
        debug!("Executing HTTP request: {} {}", 
               match request.method {
                   Method::Get => "GET",
                   Method::Post => "POST", 
                   Method::Put => "PUT",
                   Method::Delete => "DELETE",
               },
               request.url);
        
        let mut req_builder = match request.method {
            Method::Get => self.http_client.get(&request.url),
            Method::Post => self.http_client.post(&request.url),
            Method::Put => self.http_client.put(&request.url),
            Method::Delete => self.http_client.delete(&request.url),
        };
        
        // Add headers
        for (key, value) in request.headers {
            req_builder = req_builder.header(&key, &value);
        }
        
        // Add body if present
        if let Some(body) = request.body {
            req_builder = req_builder.body(body);
        }
        
        // Add cookie if present
        if let Some(cookie) = request.cookie {
            req_builder = req_builder.header("Cookie", cookie);
        }
        
        let response = req_builder.send()?;
        let status = response.status();
        let response_text = response.text()?;
        
        debug!("HTTP response status: {}", status);
        debug!("HTTP response body length: {}", response_text.len());
        
        if !status.is_success() {
            error!("HTTP request failed with status: {}", status);
            return Err(PowerShellSyncError::InvalidResponse(
                format!("HTTP request failed with status: {}", status)
            ));
        }
        
        Ok(response_text)
    }
}