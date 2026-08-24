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

#[derive(Debug)]
/// Define where the server listens and where it sources its state.
pub struct ServerConfig {
    /// The backing commit store address.
    pub commit_store_endpoint: Url,

    /// On which addresses the server listens.
    pub listen: SocketAddr,

    /// The TLS identity for the server.
    pub tls_identity: Option<(String, String)>,
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

        let tls_cert_path = std::env::var("CASTS_TLS_CERT_PATH")
            .ok()
            .or(self.tls_cert_path);
        let tls_key_path = std::env::var("CASTS_TLS_KEY_PATH")
            .ok()
            .or(self.tls_key_path);
        let server_cfg = match (tls_cert_path, tls_key_path) {
            (Some(cert_path), Some(key_path)) => {
                let cert = std::fs::read_to_string(cert_path)?;
                let key = std::fs::read_to_string(key_path)?;
                let tls_identity = Some((cert, key));
                ServerConfig {
                    commit_store_endpoint: server_url,
                    listen,
                    tls_identity,
                }
            }
            _ => ServerConfig {
                commit_store_endpoint: server_url,
                listen,
                tls_identity: None,
            },
        };
        Ok(server_cfg)
    }
}
