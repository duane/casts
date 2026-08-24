use crate::config::DEFAULT_COMMIT_STORE_URL;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use url::Url;

/// Configuring how a client connects to a remote.
#[derive(Debug, Clone)]
pub struct ClientConfig {
    /// The port of the service to contact
    pub url: Url,

    /// The CA certificate the service uses to sign the connection
    pub ca_cert: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
/// Buildinging the configuration for how a client connects to its host.
pub struct ClientConfigBuilder {
    url: Option<String>,
    ca_cert: Option<String>,
}

impl ClientConfigBuilder {
    /// Load the client configuration.
    pub fn load() -> Result<Self> {
        Ok(ClientConfigBuilder::default())
    }

    /// Construct a client configuration from the parameters passed to the builder.
    pub fn build(self) -> Result<ClientConfig> {
        // The default is the commit store's URL, not `DEFAULT_LISTEN_ADDR`: a
        // bare listen address like `::` is not a parseable URL, so the old
        // default made `ClientConfigBuilder::default().build()` always fail.
        let url = Url::parse(&self.url.unwrap_or(DEFAULT_COMMIT_STORE_URL.to_string()))?;
        Ok(ClientConfig {
            url,
            ca_cert: self.ca_cert,
        })
    }
}
