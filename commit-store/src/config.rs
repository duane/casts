extern crate serde;

use deadpool_postgres::Manager;
use deadpool_postgres::Pool;
use serde_derive::Deserialize;
use serde_derive::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use tokio_postgres::Config;

#[cfg(feature = "psql")]
use crate::postgres::PostgresStoreAsync;
use crate::AsyncStore;
#[cfg(feature = "psql")]
use native_tls::TlsConnector;
#[cfg(feature = "psql")]
use postgres_native_tls::MakeTlsConnector;

#[cfg(feature = "sqlite")]
use crate::sqlite::SqliteStore;
use crate::Store;

/// A struct to hold the backing store configuration.
#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(tag = "type", content = "config")]
pub enum StoreConfig {
    /// A filename where the sqlite database is stored.
    #[cfg(feature = "sqlite")]
    Sqlite(String),

    #[cfg(feature = "psql")]
    /// A string holding the postgres configuration (e.g. a uri)
    Postgres(String),
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct SqliteConfig {
    filename: String,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
struct PostgresConfig {
    filename: String,
}

impl StoreConfig {
    /// Load the store configuration from a given path.
    pub fn load_path(path: &Path) -> StoreConfig {
        let config = fs::read_to_string(path).expect("could not read config from path");
        toml::from_str(&config).expect("could not parse config file")
    }

    /// Load the store configuration from the default path.
    pub fn load() -> StoreConfig {
        let filename = std::env::var("STORE_CONFIG").unwrap_or(String::from("config.toml"));
        let path = PathBuf::from(filename);
        StoreConfig::load_path(&path)
    }

    /// Produce a store for the given configuration.
    pub fn configured_async_store(&self) -> Box<dyn AsyncStore> {
        match self {
            #[cfg(feature = "sqlite")]
            &StoreConfig::Sqlite(ref _file) => unimplemented!("no async sqlite store"),
            #[cfg(feature = "psql")]
            &StoreConfig::Postgres(ref config) => {
                let connector = TlsConnector::builder()
                    .danger_accept_invalid_certs(true)
                    .build()
                    .expect("failed to build tcp connector");
                let connector = MakeTlsConnector::new(connector);
                let manager: Manager = Manager::new(Config::from_str(config).unwrap(), connector);
                let pool = Pool::builder(manager).build().unwrap();
                let store = PostgresStoreAsync::with_pool(pool, config);
                Box::new(store)
            }
        }
    }

    pub fn configured_store(&self) -> Box<dyn Store> {
        match self {
            #[cfg(feature = "sqlite")]
            &StoreConfig::Sqlite(ref file) => {
                Box::new(SqliteStore::with_new_connection_at_path(Path::new(file)))
            }
            #[cfg(feature = "psql")]
            &StoreConfig::Postgres(_) => unimplemented!("postgres has no sync store"),
        }
    }
}
