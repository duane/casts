use anyhow::Result;
use commit_store::{CommitStream, DispatchingAsyncClientStore, GrpcClient};
use url::Url;
use uuid::Uuid;

use super::{entry::Entry, feed::Feed};

pub struct PostgresReadModel {
    _store: GrpcClient,
    connection: String,
    _commit_stream: CommitStream,
}

const SCHEMA: &'static str = "
    CREATE TABLE IF NOT EXISTS feeds (
        id UUID PRIMARY KEY,
        stream_id UUID NOT NULL,
        title TEXT NOT NULL,
        description TEXT NOT NULL,
        url TEXT NOT NULL
    );

    CREATE TABLE IF NOT EXISTS entries (
        id UUID PRIMARY KEY,
        feed_id UUID NOT NULL,
        guid TEXT NOT NULL,
        url TEXT NOT NULL,
        title TEXT NOT NULL,
        summary TEXT NOT NULL
    );
";

impl PostgresReadModel {
    pub async fn connect(store: GrpcClient, connection: String) -> PostgresReadModel {
        let mut store = store;
        let connection = connection;
        let commit_stream = store.subscribe_all_commits().await;
        PostgresReadModel {
            _store: store,
            connection,
            _commit_stream: commit_stream,
        }
    }

    pub async fn provision(connection: &str) -> Result<()> {
        let (client, _connection) =
            tokio_postgres::connect(connection, tokio_postgres::NoTls).await?;
        client.batch_execute(SCHEMA).await?;
        Ok(())
    }

    pub async fn list_feeds(&self, stream_id: &Uuid) -> Vec<Feed> {
        let (client, _connection) =
            tokio_postgres::connect(&self.connection, tokio_postgres::NoTls)
                .await
                .unwrap();
        client
            .query("SELECT (id AS feed_id, stream_id, title, description, url) FROM feeds WHERE stream_id = $1", &[stream_id])
            .await
            .unwrap()
            .iter()
            .map(|row| {
                let url_string: String = row.get(4);
                Feed {
                    id: row.get(0),
                    stream_id: row.get(1),
                    title: row.get(2),
                    description: row.get(3),
                    url: Url::parse(&url_string).unwrap(),
                }
            })
            .collect::<Vec<Feed>>()
    }

    pub async fn list_feed_entries(&self, stream_id: &Uuid, feed_id: &Uuid) -> Vec<Entry> {
        let (client, _connection) =
            tokio_postgres::connect(&self.connection, tokio_postgres::NoTls)
                .await
                .unwrap();
        /*
             id UUID PRIMARY KEY,
        feed_id UUID NOT NULL,
        guid TEXT NOT NULL,
        url TEXT NOT NULL,
        title TEXT NOT NULL,
        summary TEXT NOT NULL
         */
        client
            .query(
                "SELECT (id AS entry_id, feed_id, guid, url, title, summary) FROM entries WHERE stream_id = $1 feed_id = $2",
                &[stream_id, feed_id],
            )
            .await
            .unwrap()
            .iter()
            .map(|row| {
                let url_string: String = row.get(3);
                Entry {
                id: row.get(0),
                feed_id: row.get(1),
                guid: row.get(2),
                url: Url::parse(&url_string).unwrap(),
                title: row.get(4),
                summary: row.get(5),
            }})
            .collect::<Vec<Entry>>()
    }
}
