pub static DEFAULT_COMMIT_STORE_URL: &str = "http://localhost:7392";
pub static DEFAULT_PORT: u16 = 8243;
pub static DEFAULT_LISTEN_ADDR: &str = "::";
pub static DEFAULT_CONFIG_PATH: &str = "/etc/casts/config.toml";

mod client;
mod server;

pub use client::{ClientConfig, ClientConfigBuilder};
pub use server::{ServerConfig, ServerConfigBuilder};
