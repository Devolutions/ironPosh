use pwsh_core::connector::UserOperation;

/// Represents the next step in the event loop
pub enum NextStep {
    NetworkResponse(pwsh_core::connector::http::HttpResponse<String>),
    UserRequest(UserOperation),
}