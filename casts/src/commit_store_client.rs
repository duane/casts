//! One place where casts connects to the commit store.
//!
//! Every binary used to call `CommitStoreClient::connect(uri)` directly, which
//! builds a plain channel and silently ignores the client configuration's CA
//! and identity. Against a plaintext store that works; against the deployed
//! store, which requires mutual TLS, it fails with "Connecting to HTTPS
//! without TLS enabled" -- so no casts binary could reach it. commit-store's
//! `ClientConfig::tls_config` exists precisely so callers do not hand-roll
//! this; this module is the single caller.

use anyhow::{Context, Result};
use commit_store::grpc_interface::commit_store_client::CommitStoreClient;
use tonic::transport::{Channel, Endpoint, Uri};

/// Connect to the commit store named by `COMMIT_STORE_ADDR`.
///
/// TLS is applied when the address is `https`, using whatever CA and client
/// identity the environment supplies. An `http` address connects in the clear,
/// which is what local development uses.
pub async fn connect() -> Result<CommitStoreClient<Channel>> {
    let config = commit_store::config::ClientConfigBuilder::default().build();
    let addr = config.addr.clone();
    let uri: Uri = addr
        .parse()
        .with_context(|| format!("COMMIT_STORE_ADDR is not a valid URI: {addr}"))?;

    let mut endpoint = Endpoint::from_shared(addr.clone())
        .with_context(|| format!("invalid commit store endpoint: {addr}"))?;

    if uri.scheme_str() == Some("https") {
        // The domain must match the store's certificate, so take it from the
        // address rather than requiring a second variable to be kept in sync.
        let domain = uri
            .host()
            .context("https commit store address has no host")?
            .to_owned();
        endpoint = endpoint
            .tls_config(config.tls_config(&domain))
            .context("failed to apply commit store TLS configuration")?;
    }

    let channel = endpoint
        .connect()
        .await
        .with_context(|| format!("failed to connect to commit store at {addr}"))?;
    Ok(CommitStoreClient::new(channel))
}
