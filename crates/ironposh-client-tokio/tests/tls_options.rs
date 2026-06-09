//! TLS option behavior against a real self-signed TLS listener (localhost only).

use std::net::SocketAddr;
use std::sync::Arc;

use ironposh_client_core::connector::config::TlsOptions;
use ironposh_client_tokio::http_client::build_reqwest_client;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;
use tokio_rustls::rustls::pki_types::{CertificateDer, PrivateKeyDer};
use tokio_rustls::rustls::ServerConfig;
use tokio_rustls::TlsAcceptor;

const RESPONSE_401: &[u8] = b"HTTP/1.1 401 Unauthorized\r\nContent-Length: 0\r\n\r\n";

/// Spawn a TLS listener on 127.0.0.1:0 answering any request with 401.
async fn spawn_tls_server(
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
fn self_signed_localhost() -> (Vec<CertificateDer<'static>>, PrivateKeyDer<'static>) {
    let rcgen::CertifiedKey { cert, key_pair } =
        rcgen::generate_simple_self_signed(vec!["localhost".to_string()])
            .expect("self-signed cert");
    let key = PrivateKeyDer::Pkcs8(key_pair.serialize_der().into());
    (vec![cert.der().clone()], key)
}

fn wsman_url(addr: SocketAddr) -> String {
    // Hostname must match the cert SAN ("localhost"), so don't use addr.ip().
    format!("https://localhost:{}/wsman", addr.port())
}

#[tokio::test]
async fn default_tls_rejects_self_signed() {
    let (chain, key) = self_signed_localhost();
    let (addr, server) = spawn_tls_server(chain, key).await;

    let client = build_reqwest_client(&TlsOptions::default()).expect("client");
    let err = client
        .get(wsman_url(addr))
        .send()
        .await
        .expect_err("default TLS options must reject a self-signed certificate");
    assert!(
        err.is_connect(),
        "expected TLS connect failure, got: {err:?}"
    );

    server.abort();
}

#[tokio::test]
async fn insecure_tls_accepts_self_signed() {
    let (chain, key) = self_signed_localhost();
    let (addr, server) = spawn_tls_server(chain, key).await;

    let tls = TlsOptions {
        accept_invalid_certs: true,
        ..TlsOptions::default()
    };
    let client = build_reqwest_client(&tls).expect("client");
    let response = client
        .get(wsman_url(addr))
        .send()
        .await
        .expect("insecure TLS options must reach the server");
    assert_eq!(response.status().as_u16(), 401);

    server.abort();
}

#[tokio::test]
async fn extra_ca_pem_trusts_custom_ca() {
    // CA certificate.
    let mut ca_params = rcgen::CertificateParams::new(Vec::<String>::new()).expect("ca params");
    ca_params.is_ca = rcgen::IsCa::Ca(rcgen::BasicConstraints::Unconstrained);
    ca_params
        .distinguished_name
        .push(rcgen::DnType::CommonName, "ironposh test CA");
    let ca_key = rcgen::KeyPair::generate().expect("ca key");
    let ca_cert = ca_params.self_signed(&ca_key).expect("ca cert");

    // Leaf certificate for "localhost" signed by the CA.
    let mut leaf_params =
        rcgen::CertificateParams::new(vec!["localhost".to_string()]).expect("leaf params");
    leaf_params
        .distinguished_name
        .push(rcgen::DnType::CommonName, "localhost");
    let leaf_key = rcgen::KeyPair::generate().expect("leaf key");
    let leaf_cert = leaf_params
        .signed_by(&leaf_key, &ca_cert, &ca_key)
        .expect("leaf cert");

    let chain = vec![leaf_cert.der().clone(), ca_cert.der().clone()];
    let key = PrivateKeyDer::Pkcs8(leaf_key.serialize_der().into());
    let (addr, server) = spawn_tls_server(chain, key).await;

    let tls = TlsOptions {
        extra_ca_pem: Some(ca_cert.pem().into_bytes()),
        ..TlsOptions::default()
    };
    let client = build_reqwest_client(&tls).expect("client");
    let response = client
        .get(wsman_url(addr))
        .send()
        .await
        .expect("extra CA PEM must make the server trusted");
    assert_eq!(response.status().as_u16(), 401);

    server.abort();
}
