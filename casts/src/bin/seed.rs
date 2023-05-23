use casts::{
    aggregate::FeedAggregate,
    applicable::EventApplicable,
    commands::{Command, FeedAggregateCommand},
};
use chrono::Utc;
use commit_store::{
    commit::CommitAttempt, grpc_interface::commit_store_client::CommitStoreClient,
    AsyncClientStore, GrpcClient,
};
use url::Url;
use uuid::uuid;

#[tokio::main]
async fn main() {
    let stream_id = uuid!["34c3c08d-0d55-4af4-a126-170c1380f07e"];
    let commands = vec![
        // Majority Report
        (
            uuid!["7490743d-d3e6-4e47-b2e8-7d34a15786bb"],
            FeedAggregateCommand::AddFeed {
                stream_id: uuid!["cb0901a6-94a6-4fd2-8f26-709c7a6decfc"],
                feed_id: uuid!["115639fa-58d7-4049-af21-c3336a136481"],
                url: Url::parse("https://feeds.feedburner.com/MajorityReport").unwrap(),
            },
        ),
        // Hardcore History
        (
            uuid!["929e10d1-9159-478a-8baf-1c461a539fc0"],
            FeedAggregateCommand::AddFeed {
                stream_id: uuid!["95c7ea17-4ebc-4ae0-b7fa-5a8da5f2e0f2"],
                feed_id: uuid!["6a278a5d-8f37-4574-9622-c74585d02eb4"],
                url: Url::parse("http://feeds.feedburner.com/dancarlin/history?format=xml")
                    .unwrap(),
            },
        ),
        // History of Rome
        (
            uuid!["5b6ec1c2-078d-4f1c-a09b-d8c59d2c71b0"],
            FeedAggregateCommand::AddFeed {
                stream_id: uuid!["1b8fc6f0-5348-42f5-bac6-cf9bbdfbf620"],
                feed_id: uuid!["47c2f2d3-86d9-4065-8745-9b88fd877148"],
                url: Url::parse("https://historyofrome.libsyn.com/rss").unwrap(),
            },
        ),
    ];
    let mut aggregate = FeedAggregate::default();
    let mut client = GrpcClient::new(
        CommitStoreClient::connect("http://127.0.0.1:5059")
            .await
            .expect("Failed to connect to backend commit store"),
    );
    for (commit_id, cmd) in commands {
        let events_domain = cmd.async_apply(&aggregate).await.unwrap();
        let events_count = events_domain.len() as i64;
        let events = serde_json::to_value(events_domain.clone()).unwrap();

        let commit_attempt = CommitAttempt {
            stream_id,
            stream_version: aggregate.version,
            commit_id,
            commit_sequence: aggregate.version,
            commit_timestamp: Utc::now(),
            metadata: serde_json::to_value(cmd).unwrap(),
            events,
            events_count,
        };

        client.commit(&commit_attempt).await.unwrap();
        aggregate = aggregate.apply_many(events_domain);
    }
    println!("ok");
}
