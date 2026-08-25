//! Mutual TLS for the casts server, over a real TCP socket.
//!
//! Unlike `grpc_e2e.rs`, which uses an in-process duplex stream, these bind a
//! real ephemeral port: TLS is a transport concern and the handshake is the
//! thing under test. Certificates are generated at test time with `rcgen`, so
//! nothing secret is committed.
//!
//! What matters here is that the service has no authorization of its own --
//! any accepted connection gets the whole library -- so the handshake is the
//! only gate. These assert it actually closes.
#![cfg(feature = "grpc")]

use casts::grpc::Grpc2Store;
use casts::grpc_interface::casts_client::CastsClient;
use casts::grpc_interface::casts_server::CastsServer;
use casts::grpc_interface::ListFeedsRequest;
use casts::service::{build_tls_config, CastsImpl};
use commit_store::dispatch::DispatchedStore;
use commit_store::sqlite::SqliteStore;
use rcgen::{
    BasicConstraints, Certificate, CertificateParams, DnType, ExtendedKeyUsagePurpose, IsCa,
    KeyPair, KeyUsagePurpose,
};
use std::net::SocketAddr;
use std::time::Duration;
use tonic::transport::{Certificate as TonicCert, ClientTlsConfig, Identity};
use uuid::Uuid;

/// A throwaway certificate authority that can issue server and client leaves.
struct Ca {
    cert: Certificate,
    key: KeyPair,
}

impl Ca {
    fn new() -> Self {
        let mut params = CertificateParams::new(Vec::new()).unwrap();
        params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
        params.key_usages = vec![KeyUsagePurpose::KeyCertSign, KeyUsagePurpose::CrlSign];
        params
            .distinguished_name
            .push(DnType::CommonName, "casts test ca");
        let key = KeyPair::generate().unwrap();
        let cert = params.self_signed(&key).unwrap();
        Ca { cert, key }
    }

    fn pem(&self) -> String {
        self.cert.pem()
    }

    /// Issue a leaf for `name`, usable for the given purpose.
    fn issue(&self, name: &str, purpose: ExtendedKeyUsagePurpose) -> (String, String) {
        let mut params = CertificateParams::new(vec![name.to_string()]).unwrap();
        params.distinguished_name.push(DnType::CommonName, name);
        params.extended_key_usages = vec![purpose];
        let key = KeyPair::generate().unwrap();
        let cert = params.signed_by(&key, &self.cert, &self.key).unwrap();
        (cert.pem(), key.serialize_pem())
    }
}

fn free_addr() -> SocketAddr {
    std::net::TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
}

/// Start a casts server requiring client certificates from `ca`.
async fn spawn_mtls_server(ca: &Ca) -> SocketAddr {
    let addr = free_addr();
    let (cert, key) = ca.issue("localhost", ExtendedKeyUsagePurpose::ServerAuth);
    // Built by the same function `serve()` uses, so these tests cover the
    // server's real configuration path. Hand-rolling the ServerTlsConfig here
    // instead made them pass even while `build_tls_config` was dead code and
    // the server served plaintext.
    let tls = build_tls_config(Some((cert, key)), Some(ca.pem()))
        .expect("valid identity and CA")
        .expect("a configured identity yields TLS");

    let store = DispatchedStore::new(SqliteStore::with_new_in_memory_connection());
    let service = CastsServer::new(CastsImpl::new(Grpc2Store::Sqlite(store)));

    let _server = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .tls_config(tls)
            .unwrap()
            .add_service(service)
            .serve(addr)
            .await
            .expect("mtls server");
    });
    // Give the listener a moment to bind.
    tokio::time::sleep(Duration::from_millis(300)).await;
    addr
}

async fn try_list_feeds(addr: SocketAddr, tls: ClientTlsConfig) -> Result<(), String> {
    let channel = tonic::transport::Channel::from_shared(format!("https://{addr}"))
        .map_err(|e| e.to_string())?
        .tls_config(tls)
        .map_err(|e| e.to_string())?
        .connect()
        .await
        .map_err(|e| format!("connect: {e}"))?;
    let mut client = CastsClient::new(channel);
    client
        .list_feeds(ListFeedsRequest {
            stream_id: Uuid::new_v4().to_string(),
        })
        .await
        .map_err(|e| format!("rpc: {e}"))?;
    Ok(())
}

#[tokio::test]
async fn a_client_with_a_certificate_from_the_trusted_ca_is_accepted() {
    let ca = Ca::new();
    let addr = spawn_mtls_server(&ca).await;
    let (cert, key) = ca.issue("casts-client", ExtendedKeyUsagePurpose::ClientAuth);

    let tls = ClientTlsConfig::new()
        .domain_name("localhost")
        .ca_certificate(TonicCert::from_pem(ca.pem()))
        .identity(Identity::from_pem(cert, key));

    try_list_feeds(addr, tls)
        .await
        .expect("a client holding a cert from the server's CA must be served");
}

#[tokio::test]
async fn a_client_without_a_certificate_is_rejected() {
    let ca = Ca::new();
    let addr = spawn_mtls_server(&ca).await;

    // Trusts the server, presents nothing of its own.
    let tls = ClientTlsConfig::new()
        .domain_name("localhost")
        .ca_certificate(TonicCert::from_pem(ca.pem()));

    // Whether this fails at connect or at first use is TLS-stack detail; what
    // matters is that no request is ever served.
    assert!(
        try_list_feeds(addr, tls).await.is_err(),
        "a client with no certificate must not reach the service"
    );
}

#[tokio::test]
async fn a_client_with_a_certificate_from_another_ca_is_rejected() {
    let ca = Ca::new();
    let addr = spawn_mtls_server(&ca).await;

    let rogue = Ca::new();
    let (cert, key) = rogue.issue("impostor", ExtendedKeyUsagePurpose::ClientAuth);
    let tls = ClientTlsConfig::new()
        .domain_name("localhost")
        .ca_certificate(TonicCert::from_pem(ca.pem()))
        .identity(Identity::from_pem(cert, key));

    assert!(
        try_list_feeds(addr, tls).await.is_err(),
        "a valid certificate from the wrong CA must not be accepted"
    );
}

/// A client CA without a server identity must be refused outright: an
/// operator who asked for client certificates must never be silently served
/// plaintext instead.
#[test]
fn a_client_ca_without_a_server_identity_is_refused() {
    let ca = Ca::new();
    assert!(build_tls_config(None, Some(ca.pem())).is_err());
}

#[test]
fn no_tls_at_all_is_still_allowed() {
    assert!(build_tls_config(None, None).unwrap().is_none());
}
