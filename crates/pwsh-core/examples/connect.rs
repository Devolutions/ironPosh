use std::sync::Arc;

use protocol_winrm::ws_management::WsMan;
use pwsh_core::{connector::http::HttpBuilder, powershell::RunspacePool};
use tracing::info;

fn main() {
    tracing_subscriber::fmt::SubscriberBuilder::default()
        .with_target(false)
        .with_line_number(true)
        .with_max_level(tracing::Level::DEBUG)
        .init();

    let host = "10.10.0.3";

    let mut runspace_pool = RunspacePool::builder()
        .connection(Arc::new(WsMan::builder().build()))
        .build();

    let http_builder = HttpBuilder::new(
        pwsh_core::connector::http::ServerAddress::Ip(std::net::IpAddr::V4(
            std::net::Ipv4Addr::new(10, 10, 0, 3),
        )),
        5985,
        pwsh_core::connector::Scheme::Http,
        pwsh_core::connector::Authentication::Basic {
            username: "administrator".to_string(),
            password: "DevoLabs123!".to_string(),
        },
    );

    let (creation_response, expect_shell_created) =
        runspace_pool.open().expect("Failed to open runspace pool");

    info!(creation_response = ?creation_response, "Runspace pool opened message constructed");
}
