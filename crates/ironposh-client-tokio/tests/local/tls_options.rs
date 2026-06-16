//! TLS option behavior against a real self-signed TLS listener (localhost only).

use std::net::SocketAddr;

use ironposh_client_core::connector::config::TlsOptions;
use ironposh_client_tokio::http_client::build_reqwest_client;
use ironposh_test_support::tls_listener::{self_signed_localhost, spawn_tls_server};
use tokio_rustls::rustls::pki_types::PrivateKeyDer;

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
