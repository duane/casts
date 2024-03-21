// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

use crate::commit::{Commit, CommitAttempt};
use crate::{
    AsyncReadWriteStore, ClientStore, StorageCommitConflict, Store, StoreError, StoreErrorType,
};
use async_trait::async_trait;
use rusqlite::{Connection as RusqliteConnection, Error as RusqliteError, ToSql};
use std::error::Error;
use std::fmt;
use std::path::Path;
use tokio::runtime::Handle;
use tokio::sync::mpsc::{channel, Sender};
use uuid::Uuid;

/// The store representing a sqlite connection.
pub struct SqliteStore {
    conn: RusqliteConnection,
}

impl SqliteStore {
    /// create the store around the given connection
    pub fn with_connection(connection: RusqliteConnection) -> Self {
        SqliteStore { conn: connection }
    }
}

enum CallResult {
    Commit(Commit),
    Commits(Vec<Commit>),
}

type CallMessage = (
    Sender<Result<CallResult, Box<dyn StoreError>>>,
    Box<dyn (FnMut(&mut SqliteStore) -> Result<CallResult, Box<dyn StoreError>>) + Send>,
);

#[derive(Clone, Debug)]
pub struct SqliteStoreAsync {
    sender: Sender<CallMessage>,
}

impl SqliteStoreAsync {
    pub fn with_store(sqlite_store: SqliteStore) -> Self {
        let (tx, rx) = channel::<CallMessage>(10);
        let runtime_handle = Handle::current();
        let _guard = runtime_handle.enter();
        let _result = runtime_handle.spawn(async move {
            let mut sqlite_receiver = rx;
            let mut sqlite_store = sqlite_store;
            loop {
                let (result_sender, mut closure): CallMessage =
                    sqlite_receiver.recv().await.unwrap();
                let result = closure(&mut sqlite_store);
                println!("got result, vending to result sender");
                if let Err(err) = result_sender.send(result).await {
                    unimplemented!("result channel dropped: {}", err.to_string())
                }
                println!("vended!");
            }
        });

        SqliteStoreAsync { sender: tx }
    }

    pub fn with_connection(connection: RusqliteConnection) -> Self {
        Self::with_store(SqliteStore::with_connection(connection))
    }

    async fn call(
        &self,
        closure: Box<dyn FnMut(&mut SqliteStore) -> Result<CallResult, Box<dyn StoreError>> + Send>,
    ) -> Result<CallResult, Box<dyn StoreError>> {
        let (result_sender, mut result_receiver) =
            channel::<Result<CallResult, Box<dyn StoreError>>>(1);
        println!("sending");
        if let Err(_err) = self.sender.send((result_sender, closure)).await {
            panic!("Send error. :(");
        };
        println!("awaiting result");
        let result = result_receiver
            .recv()
            .await
            .expect("SqliteStoreAsync recv failure");
        println!("got result!");
        result
    }

    /// Create a new store in-memory.
    pub fn with_new_in_memory_connection() -> Self {
        let connection = RusqliteConnection::open_in_memory().unwrap();
        let mut sync_store = SqliteStore::with_connection(connection);
        sync_store.initialize().unwrap();
        Self::with_store(sync_store)
    }

    /// Create a new store at the given path.
    pub fn with_new_connection_at_path(path: &Path) -> Self {
        Self::with_connection(RusqliteConnection::open(path).unwrap())
    }
}

/// The errors returned by this store.
#[derive(Debug)]
pub struct SqliteStoreError {
    cause: RusqliteError,
}

impl fmt::Display for SqliteStoreError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "SqliteStoreError({:}, {:})",
            self.error_type(),
            self.cause
        )
    }
}

impl Error for SqliteStoreError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.cause)
    }
}

impl From<RusqliteError> for SqliteStoreError {
    fn from(cause: RusqliteError) -> Self {
        SqliteStoreError { cause }
    }
}

impl Into<Box<dyn StoreError>> for SqliteStoreError {
    fn into(self) -> Box<dyn StoreError> {
        Box::new(self)
    }
}

impl SqliteStore {
    /// Create a new store in-memory.
    pub fn with_new_in_memory_connection() -> Self {
        Self::with_connection(RusqliteConnection::open_in_memory().unwrap())
    }

    /// Create a new store at the given path.
    pub fn with_new_connection_at_path(path: &Path) -> Self {
        Self::with_connection(RusqliteConnection::open(path).unwrap())
    }
}

impl StoreError for SqliteStoreError {
    fn error_type(&self) -> StoreErrorType {
        match self.cause {
            RusqliteError::SqliteFailure(_, Some(ref msg))
                if msg
                    == "UNIQUE constraint failed: commits.stream_id, commits.commit_sequence" =>
            {
                StoreErrorType::DuplicateWriteError(StorageCommitConflict::CommitSequenceConflict)
            }
            RusqliteError::SqliteFailure(_, Some(ref msg))
                if msg == "UNIQUE constraint failed: commits.stream_id, commits.stream_version" =>
            {
                StoreErrorType::DuplicateWriteError(StorageCommitConflict::StreamVersionConflict)
            }
            RusqliteError::SqliteFailure(_, Some(ref msg))
                if msg == "UNIQUE constraint failed: commits.commit_id" =>
            {
                StoreErrorType::DuplicateWriteError(StorageCommitConflict::CommitIdConflict)
            }
            RusqliteError::SqliteFailure(_, Some(ref msg)) => {
                panic!("{}", msg.clone());
            }
            _ => StoreErrorType::UnknownError,
        }
    }
}

impl ClientStore for SqliteStore {
    fn commit(&mut self, commit_attempt: &CommitAttempt) -> Result<Commit, Box<dyn StoreError>> {
        {
            {
                let mut statement = match self.conn.prepare(
                    "INSERT INTO commits (
            stream_id,
            stream_version,
            commit_id,
            commit_timestamp,
            commit_sequence,
            events_count,
            metadata,
            events
          ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
                ) {
                    Ok(result) => result,
                    Err(err) => return Err(SqliteStoreError::from(err).into()),
                };
                match statement.execute(&[
                    &commit_attempt.stream_id.to_string(),
                    &commit_attempt.stream_version as &dyn ToSql,
                    &commit_attempt.commit_id.to_string(),
                    &commit_attempt.commit_timestamp,
                    &commit_attempt.commit_sequence,
                    &commit_attempt.events_count,
                    &serde_json::to_string(&commit_attempt.metadata)
                        .expect("could not serialize metadata to string"),
                    &serde_json::to_string(&commit_attempt.events)
                        .expect("could not serialize events to string"),
                ]) {
                    Ok(_) => (),
                    Err(err) => return Err(SqliteStoreError::from(err).into()),
                };
                match statement.finalize() {
                    Ok(_) => (),
                    Err(err) => return Err(SqliteStoreError::from(err).into()),
                };
            }
        }
        Ok(commit_attempt
            .clone()
            .into_commit(self.conn.last_insert_rowid()))
    }

    fn get_stream_range(
        &mut self,
        stream_id: Uuid,
        min_version: i64,
        max_version: i64,
    ) -> Result<Vec<Commit>, Box<dyn StoreError>> {
        println!("in get_stream_range");
        let mut stmt = match self.conn.prepare(
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
        WHERE stream_version >= ?
        AND stream_version <= ?
        AND stream_id = ?;",
        ) {
            Ok(result) => result,
            Err(err) => return Err(SqliteStoreError::from(err).into()),
        };
        let rows: Vec<Commit> = match stmt.query_map(
            &[
                &min_version,
                &max_version,
                &stream_id.to_string() as &dyn ToSql,
            ],
            |row| {
                let stream_id_str: String = row.get(0).expect("no stream_id result column");
                let commit_id_str: String = row.get(2).expect("no commit_id result column");
                let metadata_string: String =
                    row.get(7).expect("no serialized_metadat result column");
                let metadata = serde_json::from_str(&metadata_string)
                    .expect("could not deserialize metadata string");
                let events_string: String =
                    row.get(8).expect("no serialized_metadat result column");
                let events = serde_json::from_str(&events_string)
                    .expect("could not deserialize metadata string");
                Ok(Commit {
                    stream_id: Uuid::parse_str(stream_id_str.as_ref()).unwrap(),
                    stream_version: row.get(1).expect("no stream_version result column"),
                    commit_id: Uuid::parse_str(commit_id_str.as_ref()).unwrap(),
                    commit_timestamp: row.get(3).expect("no commit_timestamp result column"),
                    commit_sequence: row.get(4).expect("no commit_sequence result column"),
                    commit_number: row.get(5).expect("no commit_number result column"),
                    events_count: row.get(6).expect("no events_count result column"),
                    metadata,
                    events,
                    dispatched: row.get(9).expect("no dispatched result column"),
                })
            },
        ) {
            Ok(result) => result,
            Err(err) => return Err(SqliteStoreError::from(err).into()),
        }
        .map(|row| row.unwrap())
        .collect();
        println!("Returning {:?} rows", rows.len());
        Ok(rows)
    }

    fn get_commit(&mut self, commit_id: &Uuid) -> Result<Commit, Box<dyn StoreError>> {
        let mut statement = match self.conn.prepare(
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
        WHERE commit_id = ?
        ORDER BY commit_number ASC;",
        ) {
            Ok(result) => result,
            Err(err) => return Err(SqliteStoreError::from(err).into()),
        };
        let commit: Commit = match statement.query_row(&[&commit_id.to_string()], |row| {
            let stream_id: String = row.get(0).expect("no stream_id column in result row");
            let commit_id: String = row.get(2).expect("no commit_id column in result row");
            let metadata_string: String = row.get(7).expect("no serialized_metadats result column");
            let metadata = serde_json::from_str(&metadata_string)
                .expect("could not deserialize metadata string");
            let events_string: String = row.get(8).expect("no serialized_events result column");
            let events = serde_json::from_str(&events_string)
                .expect("could not deserialize metadata string");
            Ok(Commit {
                stream_id: Uuid::parse_str(stream_id.as_ref()).unwrap(),
                stream_version: row.get(1).expect("no stream_version column in result row"),
                commit_id: Uuid::parse_str(commit_id.as_ref()).unwrap(),
                commit_timestamp: row
                    .get(3)
                    .expect("no commit_timestamp column in result row"),
                commit_sequence: row.get(4).expect("no commit_sequence column in result row"),
                commit_number: row.get(5).expect("no commit_number column in result row"),
                events_count: row.get(6).expect("no events_count column in result row"),
                metadata,
                events,
                dispatched: row.get(9).expect("no dispatched column in result row"),
            })
        }) {
            Ok(result) => result,
            Err(err) => return Err(SqliteStoreError::from(err).into()),
        };
        Ok(commit)
    }
}

#[async_trait]
impl AsyncReadWriteStore for SqliteStoreAsync {
    async fn commit(
        &mut self,
        commit_attempt: &CommitAttempt,
    ) -> Result<Commit, Box<dyn StoreError>> {
        let attempt: CommitAttempt = commit_attempt.clone();
        let result = self
            .call(Box::new(move |sync_store: &mut SqliteStore| {
                sync_store.commit(&attempt).map(CallResult::Commit)
            }))
            .await;
        match result {
            Ok(CallResult::Commit(commit)) => Ok(commit),
            Ok(_) => unreachable!(),
            Err(err) => Err(err),
        }
    }

    async fn get_stream_range(
        &mut self,
        stream_id: Uuid,
        min_version: i64,
        max_version: i64,
    ) -> Result<Vec<Commit>, Box<dyn StoreError>> {
        let result = self
            .call(Box::new(move |sync_store: &mut SqliteStore| {
                sync_store
                    .get_stream_range(stream_id, min_version, max_version)
                    .map(CallResult::Commits)
            }))
            .await;
        match result {
            Ok(CallResult::Commits(commits)) => {
                println!("got results");
                Ok(commits)
            }
            Ok(_) => unreachable!(),
            Err(err) => Err(err),
        }
    }

    async fn get_range(
        &mut self,
        _min_commit_number: i64,
        _max_commit_number: i64,
    ) -> Result<Vec<Commit>, Box<dyn StoreError>> {
        unimplemented!("not implemented in synchronous yet either")
    }

    async fn get_commit(&mut self, commit_id: &Uuid) -> Result<Commit, Box<dyn StoreError>> {
        let commit_id = commit_id.clone();
        let result = self
            .call(Box::new(move |sync_store: &mut SqliteStore| {
                sync_store.get_commit(&commit_id).map(CallResult::Commit)
            }))
            .await;
        match result {
            Ok(CallResult::Commit(commit)) => Ok(commit),
            Ok(_) => unreachable!(),
            Err(err) => Err(err),
        }
    }
}

impl Store for SqliteStore {
    /// Provision a new database.
    fn initialize(&mut self) -> Result<(), Box<dyn StoreError>> {
        match self.conn.execute_batch(
      "CREATE TABLE IF NOT EXISTS commits (
        stream_id      VARCHAR(36) NOT NULL,
        stream_version INTEGER NOT NULL,
        commit_id         VARCHAR(36) NOT NULL,
        commit_sequence   INTEGER NOT NULL,
        commit_number     INTEGER PRIMARY KEY AUTOINCREMENT NOT NULL,
        commit_timestamp  DATETIME NOT NULL,
        events_count      INTEGER NOT NULL,
        metadata          BLOB NOT NULL,
        events            BLOB NOT NULL,
        dispatched        INTEGER NOT NULL DEFAULT 0
      );
      CREATE UNIQUE INDEX IF NOT EXISTS commits_commit_id_unique_idx ON commits (commit_id);
      CREATE UNIQUE INDEX IF NOT EXISTS commits_commit_stream_idx ON commits (stream_id, stream_version);
      CREATE UNIQUE INDEX IF NOT EXISTS commits_commit_sequence_idx ON commits (stream_id, commit_sequence);
      CREATE INDEX IF NOT EXISTS commits_dispatched_idx ON commits (dispatched);"
    ) {
      Ok(_) => Ok(()),
      Err(err) => return Err(SqliteStoreError::from(err).into()),
    }
    }

    fn get_undispatched_commits(&mut self) -> Result<Vec<Commit>, Box<dyn StoreError>> {
        let mut stmt = match self.conn.prepare(
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
        ) {
            Ok(result) => result,
            Err(err) => return Err(SqliteStoreError::from(err).into()),
        };
        let rows = match stmt
      .query_map([], |row| {
        let stream_id_str: String = row.get(0).expect("no stream_id column in result");
        let commit_id_str: String = row.get(2).expect("no commit_id column in result");

        let metadata_string: String = row.get(7).expect("no serialized_metadat result column");
        let metadata = serde_json::from_str(&metadata_string).expect("could not deserialize metadata string");
        let events_string: String = row.get(8).expect("no serialized_metadat result column");
        let events = serde_json::from_str(&events_string).expect("could not deserialize metadata string");
        Ok(Commit {
          stream_id: Uuid::parse_str(stream_id_str.as_ref())
            .expect("commit_id is not in Uuid format; database may be corrupted."),
          stream_version: row.get(1).expect("no stream_version column in result"),
          commit_id: Uuid::parse_str(commit_id_str.as_ref())
            .expect("commit_id is not in Uuid format; database may be corrupted."),
          commit_timestamp: row.get(3).expect("no commit_timestamp column in result"),
          commit_sequence: row.get(4).expect("no commit_sequence column in result"),
          commit_number: row.get(5).expect("no commit_number column in result"),
          events_count: row.get(6).expect("no events_count column in result"),
          metadata,
          events,
          dispatched: row.get(9).expect("no dispatched column in result"),
        })
      }) {
        Ok(result) => result,
        Err(err) => return Err(SqliteStoreError::from(err).into()),
      }
      .map(|rows| {
        rows.expect("Could not read from commits row. If the schema has changed, update the store to read from the appropriate format.")
      })
      .collect();
        Ok(rows)
    }

    fn mark_commit_as_dispatched(&mut self, commit_id: Uuid) -> Result<(), Box<dyn StoreError>> {
        let mut statement = match self
            .conn
            .prepare("UPDATE commits SET dispatched = 1 WHERE commit_id = ?")
        {
            Ok(result) => result,
            Err(err) => return Err(SqliteStoreError::from(err).into()),
        };
        match statement.execute(&[&commit_id.to_string()]) {
            Ok(_) => (),
            Err(err) => return Err(SqliteStoreError::from(err).into()),
        };
        match statement.finalize() {
            Ok(_) => (),
            Err(err) => return Err(SqliteStoreError::from(err).into()),
        };
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::commit::CommitAttempt;
    use crate::*;
    use chrono::Utc;
    use uuid::Uuid;
    #[test]
    fn it_allows_storing_and_retrieving_commits() {
        let mut s = sqlite::SqliteStore::with_new_in_memory_connection();
        s.initialize().unwrap();
        let commit_attempt = CommitAttempt {
            stream_id: Uuid::new_v4(),
            stream_version: 0,
            commit_id: Uuid::new_v4(),
            commit_sequence: 0,
            commit_timestamp: Utc::now(),
            events_count: 1,
            metadata: String::from("metadata").into(),
            events: vec![String::from("first")].into(),
        };
        assert_eq!(s.commit(&commit_attempt).unwrap().commit_number, 1);
        let commits = s.get_stream_range(commit_attempt.stream_id, 0, 2).unwrap();
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
            commit_sequence: commit_attempt.commit_sequence + 1,
            commit_timestamp: Utc::now(),
            events_count: 1,
            metadata: String::new().into(),
            events: vec![String::from("second")].into(),
        };
        assert_eq!(s.commit(&commit_attempt2).unwrap().commit_number, 2);

        let commits = s.get_stream_range(commit_attempt.stream_id, 0, 2).unwrap();
        assert_eq!(
            commits
                .iter()
                .map(|c| serde_json::from_value::<Vec<String>>(c.events.clone()).unwrap())
                .collect::<Vec<Vec<String>>>(),
            vec![vec![String::from("first")], vec![String::from("second")],]
        )
    }

    #[test]
    fn it_does_not_allow_double_commits_by_sequence() {
        let mut s = sqlite::SqliteStore::with_new_in_memory_connection();
        s.initialize().expect("failed to initialize database");
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
        assert_eq!(s.commit(&commit_attempt).unwrap().commit_number, 1);
        let commits = s.get_stream_range(commit_attempt.stream_id, 0, 2).unwrap();
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
            s.commit(&commit_attempt2).err().unwrap().error_type()
        );
    }

    #[test]
    fn it_does_not_allow_double_commits_by_stream_version() {
        let mut s = sqlite::SqliteStore::with_new_in_memory_connection();
        s.initialize().expect("failed to initialize database");
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
        assert_eq!(s.commit(&commit_attempt).unwrap().commit_number, 1);
        let commits = s.get_stream_range(commit_attempt.stream_id, 0, 2).unwrap();
        assert_eq!(
            commits
                .iter()
                .map(|c| serde_json::from_value::<Vec<String>>(c.events.clone()).unwrap())
                .collect::<Vec<Vec<String>>>(),
            vec![vec![String::from("first")]]
        );

        let commit_attempt2 = CommitAttempt {
            stream_id: commit_attempt.stream_id,
            stream_version: commit_attempt.stream_version,
            commit_id: Uuid::new_v4(),
            commit_sequence: commit_attempt.commit_sequence + 1,
            commit_timestamp: Utc::now(),
            events_count: 1,
            metadata: String::new().into(),
            events: vec![String::from("second")].into(),
        };
        assert_eq!(
            StoreErrorType::DuplicateWriteError(StorageCommitConflict::StreamVersionConflict),
            s.commit(&commit_attempt2).err().unwrap().error_type()
        );
    }

    #[test]
    fn it_does_not_allow_double_commits_by_commit_id() {
        let mut s = sqlite::SqliteStore::with_new_in_memory_connection();
        s.initialize().expect("failed to initialize database");
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
        assert_eq!(s.commit(&commit_attempt).unwrap().commit_number, 1);
        let commits = s.get_stream_range(commit_attempt.stream_id, 0, 2).unwrap();
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
            commit_id: commit_attempt.commit_id,
            commit_sequence: commit_attempt.commit_sequence + 1,
            commit_timestamp: Utc::now(),
            events_count: 1,
            metadata: String::new().into(),
            events: vec![String::from("second")].into(),
        };

        assert_eq!(
            StoreErrorType::DuplicateWriteError(StorageCommitConflict::CommitIdConflict),
            s.commit(&commit_attempt2).err().unwrap().error_type()
        );
    }
}
