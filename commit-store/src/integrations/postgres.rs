// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

use crate::commit::{Commit, CommitAttempt};
use crate::{AsyncReadWriteStore, AsyncStore, StorageCommitConflict, StoreError, StoreErrorType};
use anyhow::Result;
use async_trait::async_trait;
use deadpool_postgres::{Manager, Pool};
use std::error::Error;
use std::fmt::Debug;
use std::fmt::{self, Display};
use std::str::FromStr;
use tokio_postgres::{Config, NoTls};
use url::Url;
use uuid::Uuid;

/// The errors returned by this store.
#[derive(Debug)]
pub enum PostgresStoreError {
    // SyncError(postgres::Error),
    AsyncError(tokio_postgres::Error),
}

impl fmt::Display for PostgresStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            // PostgresStoreError::SyncError(err) => write!(
            //     f,
            //     "PostgresStoreError({:}, {:})",
            //     StoreErrorType::UnknownError,
            //     err,
            // ),
            PostgresStoreError::AsyncError(err) => {
                write!(f, "PostgresStoreError({:}, {:})", self.error_type(), err)
            }
        }
    }
}

impl Error for PostgresStoreError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            // PostgresStoreError::SyncError(err) => Some(err),
            PostgresStoreError::AsyncError(err) => Some(err),
        }
    }
}

// impl From<postgres::Error> for PostgresStoreError {
//     fn from(cause: postgres::Error) -> Self {
//         PostgresStoreError::SyncError(cause)
//     }
// }

impl From<tokio_postgres::Error> for PostgresStoreError {
    fn from(cause: tokio_postgres::Error) -> Self {
        PostgresStoreError::AsyncError(cause)
    }
}

impl Into<Box<dyn StoreError>> for PostgresStoreError {
    fn into(self) -> Box<dyn StoreError> {
        Box::new(self)
    }
}

impl StoreError for PostgresStoreError {
    fn error_type(&self) -> StoreErrorType {
        match self {
            PostgresStoreError::AsyncError(err) => match err.as_db_error() {
                Some(db_err) => {
                    if db_err.message().starts_with("duplicate key value violates unique constraint \"commits_commit_stream_idx\"") {
                        return StoreErrorType::DuplicateWriteError(StorageCommitConflict::StreamVersionConflict)
                    }
                    if db_err.message().starts_with("duplicate key value violates unique constraint \"commits_commit_id_unique_idx\"") {
                        return StoreErrorType::DuplicateWriteError(StorageCommitConflict::CommitIdConflict)
                    }
                    if db_err.message().starts_with("duplicate key value violates unique constraint \"commits_commit_sequence_idx\"") {
                        return StoreErrorType::DuplicateWriteError(StorageCommitConflict::CommitSequenceConflict)
                    }
                }
                None => (),
            },
        }
        StoreErrorType::UnknownError
    }
}

#[derive(Clone)]
/// The store representing a postgres connection.
pub struct PostgresStoreAsync {
    pool: deadpool_postgres::Pool,
    connect_str: String,
}

impl Debug for PostgresStoreAsync {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("PostgresStoreAsync({:?})", self.connect_str))
    }
}

impl PostgresStoreAsync {
    /// create the store around the given client
    pub async fn with_connection(connect_str: String) -> Self {
        let config = Config::from_str(&connect_str).unwrap();
        let manager = Manager::new(config, NoTls);
        let pool = Pool::builder(manager).build().unwrap();

        PostgresStoreAsync { pool, connect_str }
    }

    pub fn with_pool(pool: Pool, connect_str: &str) -> PostgresStoreAsync {
        PostgresStoreAsync {
            pool,
            connect_str: connect_str.to_owned(),
        }
    }
}

#[async_trait]
impl AsyncReadWriteStore for PostgresStoreAsync {
    async fn commit(
        &mut self,
        commit_attempt: &CommitAttempt,
    ) -> Result<Commit, Box<dyn StoreError>> {
        let mut client = self.pool.get().await.unwrap();
        let transaction = match client.transaction().await {
            Ok(result) => result,
            Err(err) => return Err(PostgresStoreError::from(err).into()),
        };
        let commit_number = match transaction
            .query(
                "INSERT INTO commits (
                            stream_id,
                            stream_version,
                            commit_id,
                            commit_timestamp,
                            commit_sequence,
                            events_count,
                            metadata,
                            events
                        ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
                        RETURNING commit_number",
                &[
                    &commit_attempt.stream_id,
                    &(commit_attempt.stream_version as i32),
                    &commit_attempt.commit_id,
                    &commit_attempt.commit_timestamp,
                    &(commit_attempt.commit_sequence as i32),
                    &(commit_attempt.events_count as i32),
                    &commit_attempt.metadata,
                    &commit_attempt.events,
                ],
            )
            .await
        {
            Ok(result) => result.get(0).expect("did not commit").get::<usize, i32>(0) as i64,
            Err(err) => return Err(PostgresStoreError::from(err).into()),
        };
        match transaction.commit().await {
            Ok(_) => Ok(commit_attempt.clone().into_commit(commit_number)),
            Err(err) => Err(PostgresStoreError::from(err).into()),
        }
    }

    async fn get_stream_range(
        &mut self,
        stream_id: Uuid,
        min_version: i64,
        max_version: i64,
    ) -> Result<Vec<Commit>, Box<dyn StoreError>> {
        let mut client = self.pool.get().await.unwrap();
        let transaction = match client.transaction().await {
            Ok(result) => result,
            Err(err) => return Err(PostgresStoreError::from(err).into()),
        };
        let rows = match transaction
            .query(
                "SELECT
          stream_id,
          stream_version,
          commit_id,
          commit_timestamp,
          commit_sequence,
          commit_number,
          events_count,
          metadata,
          events,
          dispatched
        FROM commits
        WHERE stream_version >= $1
        AND stream_version <= $2
        AND stream_id = $3;",
                &[
                    &(min_version as i32),
                    &(std::cmp::min(max_version, i32::max_value() as i64) as i32),
                    &stream_id,
                ],
            )
            .await
        {
            Ok(rows) => rows,
            Err(err) => return Err(PostgresStoreError::from(err).into()),
        };
        Ok(rows
            .into_iter()
            .map(|row| Commit {
                stream_id: row.get(0),
                stream_version: row.get::<usize, i32>(1) as i64,
                commit_id: row.get(2),
                commit_timestamp: row.get(3),
                commit_sequence: row.get::<usize, i32>(4) as i64,
                commit_number: row.get::<usize, i32>(5) as i64,
                events_count: row.get::<usize, i32>(6) as i64,
                metadata: row.get(7),
                events: row.get(8),
                dispatched: row.get(9),
            })
            .collect())
    }

    async fn get_range(
        &mut self,
        min_commit_number: i64,
        max_commit_number: i64,
    ) -> Result<Vec<Commit>, Box<dyn StoreError>> {
        let mut client = self.pool.get().await.unwrap();
        let transaction = match client.transaction().await {
            Ok(result) => result,
            Err(err) => return Err(PostgresStoreError::from(err).into()),
        };
        let rows = match transaction
            .query(
                "SELECT
          stream_id,
          stream_version,
          commit_id,
          commit_timestamp,
          commit_sequence,
          commit_number,
          events_count,
          metadata,
          events,
          dispatched
        FROM commits
        WHERE commit_number > $1
        AND commit_number <= $2;",
                &[
                    &(min_commit_number as i32),
                    &(std::cmp::min(max_commit_number, i32::max_value() as i64) as i32),
                ],
            )
            .await
        {
            Ok(rows) => rows,
            Err(err) => return Err(PostgresStoreError::from(err).into()),
        };
        Ok(rows
            .into_iter()
            .map(|row| Commit {
                stream_id: row.get(0),
                stream_version: row.get::<usize, i32>(1) as i64,
                commit_id: row.get(2),
                commit_timestamp: row.get(3),
                commit_sequence: row.get::<usize, i32>(4) as i64,
                commit_number: row.get::<usize, i32>(5) as i64,
                events_count: row.get::<usize, i32>(6) as i64,
                metadata: row.get(7),
                events: row.get(8),
                dispatched: row.get(9),
            })
            .collect())
    }

    async fn get_commit(&mut self, commit_id: &Uuid) -> Result<Commit, Box<dyn StoreError>> {
        let mut client = self.pool.get().await.unwrap();
        let transaction = match client.transaction().await {
            Ok(result) => result,
            Err(err) => return Err(PostgresStoreError::from(err).into()),
        };
        let rows = match transaction
            .query(
                "SELECT
          stream_id,
          stream_version,
          commit_id,
          commit_timestamp,
          commit_sequence,
          commit_number,
          events_count,
          metadata,
          events,
          dispatched
        FROM commits
        WHERE commit_id = $1
        ORDER BY commit_number ASC;",
                &[&commit_id],
            )
            .await
        {
            Ok(rows) => rows,
            Err(err) => return Err(PostgresStoreError::from(err).into()),
        };
        assert_eq!(1, rows.len());
        let row = rows.get(0).expect("query did not find any rows");
        Ok(Commit {
            stream_id: row.get(0),
            stream_version: row.get::<usize, i32>(1) as i64,
            commit_id: row.get(2),
            commit_timestamp: row.get(3),
            commit_sequence: row.get::<usize, i32>(4) as i64,
            commit_number: row.get::<usize, i32>(5) as i64,
            events_count: row.get::<usize, i32>(6) as i64,
            metadata: row.get(7),
            events: row.get(8),
            dispatched: row.get(9),
        })
    }
}

#[async_trait]
impl AsyncStore for PostgresStoreAsync {
    /// Provision a new database.
    async fn initialize(&mut self) -> Result<(), Box<dyn StoreError>> {
        let client = self.pool.get().await.unwrap();
        match client.batch_execute(
            "CREATE TABLE IF NOT EXISTS commits (
                stream_id      UUID NOT NULL,
                stream_version INTEGER NOT NULL,
                commit_id         UUID NOT NULL,
                commit_sequence   INTEGER NOT NULL,
                commit_number     SERIAL PRIMARY KEY,
                commit_timestamp  TIMESTAMP WITH TIME ZONE NOT NULL,
                events_count      INTEGER NOT NULL,
                metadata          JSON NOT NULL,
                events            JSON NOT NULL,
                dispatched        BOOLEAN NOT NULL DEFAULT FALSE
            );
            CREATE UNIQUE INDEX IF NOT EXISTS commits_commit_id_unique_idx ON commits (commit_id);
            CREATE UNIQUE INDEX IF NOT EXISTS commits_commit_stream_idx ON commits (stream_id, stream_version);
            CREATE UNIQUE INDEX IF NOT EXISTS commits_commit_sequence_idx ON commits (stream_id, commit_sequence);
            CREATE INDEX IF NOT EXISTS commits_dispatched_idx ON commits (dispatched);"
        ).await {
            Ok(_) => Ok(()),
            Err(err) => Err(PostgresStoreError::from(err).into()),
        }
    }

    async fn get_undispatched_commits(&mut self) -> Result<Vec<Commit>, Box<dyn StoreError>> {
        let mut client = self.pool.get().await.unwrap();
        let transaction = match client.transaction().await {
            Ok(result) => result,
            Err(err) => return Err(PostgresStoreError::from(err).into()),
        };
        let rows = match transaction
            .query(
                "SELECT
          stream_id,
          stream_version,
          commit_id,
          commit_timestamp,
          commit_sequence,
          commit_number,
          events_count,
          metadata,
          events,
          dispatched
        FROM commits
        WHERE dispatched = 0
        ORDER BY commit_number ASC;",
                &[],
            )
            .await
        {
            Ok(rows) => rows,
            Err(err) => return Err(PostgresStoreError::from(err).into()),
        };
        Ok(rows
            .into_iter()
            .map(|row| Commit {
                stream_id: row.get(0),
                stream_version: row.get::<usize, i32>(1) as i64,
                commit_id: row.get(2),
                commit_timestamp: row.get(3),
                commit_sequence: row.get::<usize, i32>(4) as i64,
                commit_number: row.get::<usize, i32>(5) as i64,
                events_count: row.get::<usize, i32>(6) as i64,
                metadata: row.get(7),
                events: row.get(8),
                dispatched: row.get(9),
            })
            .collect())
    }

    async fn mark_commit_as_dispatched(
        &mut self,
        commit_id: Uuid,
    ) -> Result<(), Box<dyn StoreError>> {
        let mut client = self.pool.get().await.unwrap();
        let transaction = match client.transaction().await {
            Ok(result) => result,
            Err(err) => return Err(PostgresStoreError::from(err).into()),
        };
        match transaction
            .execute(
                "UPDATE commits SET dispatched = 1 WHERE commit_id = $1",
                &[&commit_id.to_string()],
            )
            .await
        {
            Ok(_) => (),
            Err(err) => return Err(PostgresStoreError::from(err).into()),
        };
        match transaction.commit().await {
            Ok(_) => Ok(()),
            Err(err) => Err(PostgresStoreError::from(err).into()),
        }
    }
}

#[derive(Debug, PartialEq)]
enum PostgresUriError {
    WrongScheme,
    NoHost,
}

impl Display for PostgresUriError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!("{:?}", &self))
    }
}

impl Error for PostgresUriError {}

pub fn postgres_uri_to_connection_str(uri: &str) -> Result<String> {
    let parsed = Url::parse(uri)?;
    if parsed.scheme() != "postgresql" {
        return Err(PostgresUriError::WrongScheme.into());
    }
    let mut params: Vec<String> = vec![];

    match parsed.host() {
        Some(host) => params.push(format!("host={}", host)),
        None => return Err(PostgresUriError::NoHost.into()),
    }

    if !parsed.username().is_empty() {
        params.push(format!("user={}", parsed.username()));
    }

    if parsed.path().is_empty() {
        params.push(format!("dbname={}", parsed.path()));
    } else {
        params.push(format!("dbname=postgres"));
    }

    match parsed.port() {
        Some(port) => params.push(format!("port={}", port)),
        None => (),
    }

    Ok(params.join(" "))
}

#[cfg(test)]
mod tests {
    const CONNECTION_URI: &str = "postgresql://duane@localhost/postgres";

    use chrono::Utc;
    use postgres::{Client, NoTls};
    use uuid::Uuid;

    use crate::postgres::{postgres_uri_to_connection_str, PostgresUriError};
    use crate::{commit::CommitAttempt, StorageCommitConflict, StoreErrorType};

    use super::PostgresStoreAsync;

    #[test]
    fn parses_uri_ok() {
        let result = postgres_uri_to_connection_str("https://example.com");
        assert!(result.is_err());
        if let Some(e) = result.unwrap_err().downcast_ref::<PostgresUriError>() {
            assert_eq!(e, &PostgresUriError::WrongScheme)
        }

        assert_eq!(
            "host=localhost user=duane dbname=postgres",
            postgres_uri_to_connection_str(CONNECTION_URI).unwrap()
        );
    }

    struct PostgresTestConnection {
        name: String,
        store: PostgresStoreAsync,
    }

    impl PostgresTestConnection {
        fn with_store(store: PostgresStoreAsync) -> Self {
            let mut store = store;
            let name = "test-database";
            store
                .client
                .batch_execute("CREATE DATABASE \"test-database\";")
                .unwrap();
            PostgresTestConnection {
                name: "test-database".to_string(),
                store,
            }
        }
    }

    impl Drop for PostgresTestConnection {
        fn drop(&mut self) {
            self.store
                .client
                .batch_execute("DROP DATABASE \"test-database\";")
                .unwrap();
        }
    }

    #[test]
    // #[ignore]
    fn expensive_postgres_test() {
        let mut tmp_database =
            PostgresTestConnection::with_store(PostgresStoreSync::with_connection(
                Client::connect(
                    postgres_uri_to_connection_str(CONNECTION_URI)
                        .unwrap()
                        .as_str(),
                    NoTls,
                )
                .unwrap(),
            ));
        tmp_database.store.initialize().unwrap();

        let commit_attempt = CommitAttempt {
            stream_id: Uuid::new_v4(),
            stream_version: 0,
            commit_id: Uuid::new_v4(),
            commit_sequence: 0,
            commit_timestamp: Utc::now(),
            events_count: 1,
            metadata: String::new().into(),
            events: vec![String::from("first")].into(),
        };
        // assert_eq!(tmp_database.store.commit(&commit_attempt).unwrap(), 1);
        let commits = tmp_database
            .store
            .get_stream_range(commit_attempt.stream_id, 0, 2)
            .unwrap();
        assert_eq!(
            commits
                .iter()
                .map(|c| serde_json::from_value::<Vec<String>>(c.events.clone()).unwrap())
                .collect::<Vec<Vec<String>>>(),
            vec![vec![String::from("first")]]
        );

        let commit_attempt2 = CommitAttempt {
            stream_id: commit_attempt.stream_id,
            stream_version: commit_attempt.stream_version + 1,
            commit_id: Uuid::new_v4(),
            commit_sequence: commit_attempt.commit_sequence,
            commit_timestamp: Utc::now(),
            events_count: 1,
            metadata: String::new().into(),
            events: vec![String::from("second")].into(),
        };

        assert_eq!(
            StoreErrorType::DuplicateWriteError(StorageCommitConflict::CommitSequenceConflict),
            tmp_database
                .store
                .commit(&commit_attempt2)
                .err()
                .unwrap()
                .error_type()
        );
    }
}
