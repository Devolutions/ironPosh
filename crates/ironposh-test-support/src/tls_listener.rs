//! Local self-signed TLS listener helpers.
//!
//! Shared by client-tokio's WSMan TLS tests (`tests/tls_options.rs`) and the
//! KDC TLS unit tests (`src/http_client.rs`), which exercise different client
//! paths against the same listener setup.

use std::net::SocketAddr;
use std::sync::Arc;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer};
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::TlsAcceptor;

const RESPONSE_401: &[u8] = b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\n\r\n";

/// Spawn a TLS listener on 127.0.0.1:0 answering any request with 401.
pub async fn spawn_tls_server(
    cert_chain: Vec<CertificateDer<'static>>,
    key: PrivateKeyDer<'static>,
) -> (SocketAddr, tokio::task::JoinHandle<()>) {
    let provider = Arc::new(tokio_rustls::rustls::crypto::ring::default_provider());
    let config = ServerConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .expect("protocol versions")
        .with_no_client_auth()
        .with_single_cert(cert_chain, key)
        .expect("server config");
    let acceptor = TlsAcceptor::from(Arc::new(config));

    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("local addr");

    let handle = tokio::spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                return;
            };
            let acceptor = acceptor.clone();
            tokio::spawn(async move {
                let Ok(mut tls) = acceptor.accept(stream).await else {
                    return;
                };
                let mut buf = [0_u8; 4096];
                let _ = tls.read(&mut buf).await;
                let _ = tls.write_all(RESPONSE_401).await;
                let _ = tls.shutdown().await;
            });
        }
    });

    (addr, handle)
}

/// Self-signed cert for "localhost".
pub fn self_signed_localhost() -> (Vec<CertificateDer<'static>>, PrivateKeyDer<'static>) {
    let rcgen::CertifiedKey { cert, key_pair } =
        rcgen::generate_simple_self_signed(vec!["localhost".to_string()])
            .expect("self-signed cert");
    let key = PrivateKeyDer::Pkcs8(key_pair.serialize_der().into());
    (vec![cert.der().clone()], key)
}
