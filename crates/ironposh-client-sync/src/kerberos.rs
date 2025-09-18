use anyhow::Context;
use byteorder::{BigEndian, ReadBytesExt};
use ironposh_client_core::connector::NetworkRequest;
use std::io::{Read, Write};
use std::net::TcpStream;
use tracing::{info, instrument};

/// Sends a network packet to the Kerberos KDC and returns the response
#[instrument(
    name = "kerberos.send_packet",
    level = "info",
    skip(packet),
    fields(protocol = ?packet.protocol, url = %packet.url, data_len = packet.data.len()),
    err
)]
pub fn send_packet(packet: NetworkRequest) -> Result<Vec<u8>, anyhow::Error> {
    info!("sending packet to KDC");

    match packet.protocol {
        ironposh_client_core::connector::NetworkProtocol::Tcp => send_tcp_packet(packet),
        ironposh_client_core::connector::NetworkProtocol::Udp => {
            todo!("UDP protocol not implemented for Kerberos")
        }
        ironposh_client_core::connector::NetworkProtocol::Http => {
            todo!("HTTP protocol not implemented for Kerberos")
        }
        ironposh_client_core::connector::NetworkProtocol::Https => {
            todo!("HTTPS protocol not implemented for Kerberos")
        }
    }
}

/// Sends a packet via TCP to the Kerberos KDC
#[instrument(
    name = "kerberos.tcp",
    level = "info",
    skip(packet),
    fields(host = packet.url.host_str(), port = packet.url.port()),
    err
)]
fn send_tcp_packet(packet: NetworkRequest) -> Result<Vec<u8>, anyhow::Error> {
    let host = packet
        .url
        .host_str()
        .ok_or_else(|| anyhow::anyhow!("Missing host in URL"))?;
    let port = packet
        .url
        .port()
        .ok_or_else(|| anyhow::anyhow!("Missing port in URL"))?;

    info!("establishing TCP connection to KDC");

    // Establish TCP connection to the KDC
    let mut stream =
        TcpStream::connect((host, port)).context("failed to establish TCP connection to KDC")?;

    // Send the packet data
    stream
        .write_all(&packet.data)
        .context("failed to write packet data to KDC")?;

    stream
        .flush()
        .context("failed to flush TCP stream to KDC")?;

    info!("packet sent, waiting for KDC response");

    // Read the response length (4 bytes, big-endian)
    let response_len = stream
        .read_u32::<BigEndian>()
        .context("failed to read response length from KDC")?;

    // Read the response data
    let mut response_data = vec![0u8; response_len as usize + 4];
    response_data[..4].copy_from_slice(&response_len.to_be_bytes()); // include length prefix

    stream
        .read_exact(&mut response_data[4..])
        .context("failed to read response data from KDC")?;

    info!(
        response_len = response_data.len(),
        "received response from KDC"
    );

    Ok(response_data)
}
