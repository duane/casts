//! Standalone `grpc.health.v1.Health/Check` client, meant to run as an ECS
//! container health check command.
//!
//! An NLB target group can only do a TCP connect check -- gRPC-protocol
//! health checks are an ALB-only feature, and casts sits behind an NLB so
//! that client certificates reach it intact. A TCP check reports a task
//! healthy purely because the port accepts connections, even when the
//! process cannot reach the commit store and every RPC would fail. ECS's
//! container-level health check runs a command inside the container and
//! reflects the result to the load balancer target, which is what actually
//! closes that gap.
//!
//! Mirrors commit-store's `health-probe`, including why it does its own PEM
//! materialization: Docker execs `HEALTHCHECK`'s command directly rather
//! than through `ENTRYPOINT`, so the entrypoint's `*_PEM`-to-file conversion
//! never runs for it.

use anyhow::{bail, Result};
use commit_store::config::ClientConfig;
use tonic_health::pb::health_check_response::ServingStatus;
use tonic_health::pb::health_client::HealthClient;
use tonic_health::pb::HealthCheckRequest;

/// If `pem_var` is set and `path_var` is not, write `pem_var`'s content to
/// `out_path` and point `path_var` at it.
///
/// ECS injects Secrets Manager content as an environment variable, but
/// feeding PEM content straight into tonic/rustls from an env-sourced
/// `String` reproducibly breaks the mTLS handshake with an HTTP/2 framing
/// error, while file delivery has never failed. See commit-store's
/// `health-probe` for the full history.
fn materialize_pem_if_needed(pem_var: &str, path_var: &str, out_path: &str) -> Result<()> {
    if std::env::var(path_var).is_ok() {
        return Ok(());
    }
    let Ok(content) = std::env::var(pem_var) else {
        return Ok(());
    };
    let path = std::path::Path::new(out_path);
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(path, content)?;
    std::env::set_var(path_var, out_path);
    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    // The probe needs its own identity, from the *casts* intermediate.
    //
    // It connects to casts, and casts verifies client certificates against
    // the casts CA -- so the commit-store client identity this process also
    // carries (issued by the commit intermediate, for talking to
    // commit-store) would be rejected. Two intermediates means two
    // identities, and the probe must use the right one or every task fails
    // its own health check.
    //
    // `ClientConfig` reads the COMMIT_STORE_* names, so the CASTS_HEALTH_*
    // values are materialized to files and those names pointed at them.
    // Safe because this is a short-lived process of its own.
    materialize_pem_if_needed(
        "CASTS_HEALTH_CA_PEM",
        "COMMIT_STORE_CA_PEM_PATH",
        "/tmp/casts-health-tls/ca.pem",
    )?;
    materialize_pem_if_needed(
        "CASTS_HEALTH_CLIENT_CERT_PEM",
        "COMMIT_STORE_CLIENT_CERT",
        "/tmp/casts-health-tls/client-cert.pem",
    )?;
    materialize_pem_if_needed(
        "CASTS_HEALTH_CLIENT_KEY_PEM",
        "COMMIT_STORE_CLIENT_KEY",
        "/tmp/casts-health-tls/client-key.pem",
    )?;

    let port: u16 = std::env::var("CASTS_PORT").map_or(casts::config::DEFAULT_PORT, |s| {
        s.parse()
            .unwrap_or_else(|_| panic!("failed to parse CASTS_PORT {s:?}"))
    });

    // Same signal `ServerConfigBuilder::build` uses to decide whether TLS is
    // on; both forms must be checked, since which one is set depends on
    // whether the identity came from a file or from ECS's injected content.
    let tls_on =
        std::env::var("CASTS_TLS_CERT_PATH").is_ok() || std::env::var("CASTS_TLS_CERT_PEM").is_ok();
    let scheme = if tls_on { "https" } else { "http" };
    let addr = format!("{scheme}://127.0.0.1:{port}");

    // Verified against the name on the server's certificate, not the socket
    // address dialed. casts' server cert carries a `localhost` SAN precisely
    // so this probe can connect over the loopback interface.
    let domain_name =
        std::env::var("CASTS_HEALTH_TLS_DOMAIN").unwrap_or_else(|_| "localhost".to_string());

    let mut endpoint = tonic::transport::Channel::from_shared(addr)?
        .timeout(std::time::Duration::from_secs(3))
        .connect_timeout(std::time::Duration::from_secs(3));
    if tls_on {
        endpoint = endpoint.tls_config(ClientConfig::default().tls_config(&domain_name))?;
    }
    let channel = endpoint.connect().await?;

    let response = HealthClient::new(channel)
        .check(HealthCheckRequest {
            service: String::new(),
        })
        .await?;

    if response.into_inner().status() == ServingStatus::Serving {
        Ok(())
    } else {
        bail!("service is not SERVING");
    }
}
