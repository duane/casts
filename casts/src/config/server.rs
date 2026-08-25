use crate::config::{
    DEFAULT_COMMIT_STORE_URL, DEFAULT_CONFIG_PATH, DEFAULT_LISTEN_ADDR, DEFAULT_PORT,
};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::{
    net::{IpAddr, SocketAddr},
    str::FromStr,
};
use url::Url;

/// Define where the server listens and where it sources its state.
pub struct ServerConfig {
    /// The backing commit store address.
    pub commit_store_endpoint: Url,

    /// On which addresses the server listens.
    pub listen: SocketAddr,

    /// The server's own TLS certificate and private key, as PEM.
    pub tls_identity: Option<(String, String)>,

    /// PEM of the certificate authority that client certificates must chain
    /// to.
    ///
    /// Setting this turns on mutual TLS: a client must present a certificate
    /// signed by this CA or the connection is refused during the handshake,
    /// before any RPC is dispatched. This service has no authorization of its
    /// own -- every caller gets the whole library -- so on any network that
    /// is not loopback, this is what stands between the library and whoever
    /// finds the port.
    pub client_ca: Option<String>,
}

/// Hand-written so the private key cannot reach a log: `tls_identity` holds a
/// PEM-encoded key, and a derived `Debug` would print it in full. Same
/// reasoning as commit-store's `ServerConfig`.
impl std::fmt::Debug for ServerConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ServerConfig")
            .field("commit_store_endpoint", &self.commit_store_endpoint)
            .field("listen", &self.listen)
            .field(
                "tls_identity",
                &self
                    .tls_identity
                    .as_ref()
                    .map(|_| "<cert and key redacted>"),
            )
            .field("client_ca", &self.client_ca.as_ref().map(|_| "<ca pem>"))
            .finish()
    }
}

impl ServerConfig {
    /// Load the server configuration from the environment and the config file.
    pub fn load() -> Result<ServerConfig> {
        ServerConfigBuilder::load().and_then(|builder| builder.build())
    }
}

/// The builder for the server configuration
#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct ServerConfigBuilder {
    commit_store_endpoint: Option<String>,
    listen_port: Option<u16>,
    listen_addr: Option<String>,
    tls_cert_path: Option<String>,
    tls_key_path: Option<String>,
    /// Path to the CA that client certificates must chain to. Setting it
    /// enables mutual TLS.
    client_ca_path: Option<String>,
}

/// Read a PEM either from a file path or, failing that, straight out of an
/// environment variable.
///
/// The content-direct form exists because ECS injects a Secrets Manager value
/// as an environment variable, not a file, and Fargate has no secret-volume
/// equivalent -- commit-store's deploy hit exactly this. An explicit config
/// value wins over `*_PATH`, which wins over `*_PEM`.
fn read_pem(configured: Option<String>, path_var: &str, pem_var: &str) -> Result<Option<String>> {
    let path = configured.or_else(|| std::env::var(path_var).ok());
    if let Some(path) = path {
        return Ok(Some(std::fs::read_to_string(&path).map_err(|err| {
            anyhow::anyhow!("failed to read {path_var} ({path}): {err}")
        })?));
    }
    Ok(std::env::var(pem_var).ok())
}

impl ServerConfigBuilder {
    fn config_path() -> String {
        std::env::var("CASTS_CONFIG_PATH").unwrap_or_else(|_| DEFAULT_CONFIG_PATH.to_string())
    }

    pub fn load() -> Result<ServerConfigBuilder> {
        let path = ServerConfigBuilder::config_path();
        let builder = if let Ok(contents) = std::fs::read_to_string(path) {
            let builder: ServerConfigBuilder = toml::from_str(&contents)?;
            builder
        } else {
            ServerConfigBuilder::default()
        };
        Ok(builder)
    }

    pub fn build(self) -> Result<ServerConfig> {
        let server_url =
            Url::from_str(&std::env::var("COMMIT_STORE_SERVER").unwrap_or_else(|_| {
                self.commit_store_endpoint
                    .unwrap_or_else(|| DEFAULT_COMMIT_STORE_URL.to_string())
            }))?;

        let listen_port = std::env::var("CASTS_PORT")
            .map(|s| s.parse().unwrap())
            .unwrap_or_else(|_| self.listen_port.unwrap_or(DEFAULT_PORT));

        let listen_address = std::env::var("CASTS_LISTEN")
            .map(|s| IpAddr::from_str(&s).unwrap())
            .unwrap_or_else(|_| {
                self.listen_addr
                    .map(|s| IpAddr::from_str(&s).unwrap())
                    .unwrap_or_else(|| IpAddr::from_str(DEFAULT_LISTEN_ADDR).unwrap())
            });

        let listen = SocketAddr::new(listen_address, listen_port);

        let cert = read_pem(
            self.tls_cert_path,
            "CASTS_TLS_CERT_PATH",
            "CASTS_TLS_CERT_PEM",
        )?;
        let key = read_pem(self.tls_key_path, "CASTS_TLS_KEY_PATH", "CASTS_TLS_KEY_PEM")?;
        let tls_identity = match (cert, key) {
            (Some(cert), Some(key)) => Some((cert, key)),
            // Half an identity is always a misconfiguration; say so rather
            // than silently serving plaintext.
            (Some(_), None) => {
                anyhow::bail!("a TLS certificate is configured but its key is not")
            }
            (None, Some(_)) => {
                anyhow::bail!("a TLS key is configured but its certificate is not")
            }
            (None, None) => None,
        };

        let client_ca = read_pem(
            self.client_ca_path,
            "CASTS_CLIENT_CA_PATH",
            "CASTS_CLIENT_CA_PEM",
        )?;

        Ok(ServerConfig {
            commit_store_endpoint: server_url,
            listen,
            tls_identity,
            client_ca,
        })
    }
}
