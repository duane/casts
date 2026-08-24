use casts::{aggregate::FeedAggregate, applicable::EventApplicable, events::FeedAggregateEvent};
use chrono::Utc;
use commit_store::{commit::StreamMetadata, store::Store};
use commit_store::{
    commit::{CommitAttempt, CommitMetadata},
    grpc_interface::commit_store_client::CommitStoreClient,
    Client,
};
use serde_json::json;
use uuid::uuid;

#[tokio::main]
async fn main() {
    let commit_attempts = vec![
        CommitAttempt {
            stream_metadata: StreamMetadata {
                stream_id: uuid!["34c3c08d-0d55-4af4-a126-170c1380f07e"],
                stream_version: 0,
            },
            commit_metadata: CommitMetadata {
                commit_id: uuid!["7490743d-d3e6-4e47-b2e8-7d34a15786bb"],
                commit_sequence: 0,
                commit_timestamp: Utc::now(),
            },
            metadata: json!({"AddFeed":{"feed_id":"115639fa-58d7-4049-af21-c3336a136481","stream_id":"34c3c08d-0d55-4af4-a126-170c1380f07e","url":"https://feeds.feedburner.com/MajorityReport"}}),
            events: json!([{"Added":{"entries":[],"feed_id":"115639fa-58d7-4049-af21-c3336a136481","metadata":{"authors":["Sam Seder"],"description":"Entertaining Daily Politics, Award Winning Long-Form Interviews and Irreverent, Independent Analysis.  For more info see Majority.FM","logo":{"description":null,"height":null,"link":{"href":"http://majorityfm.libsyn.com/","href_lang":null,"length":null,"media_type":null,"rel":null,"title":null},"raw_url":"https://static.libsyn.com/p/assets/1/2/7/5/1275caa5c11b3ea55f2e77a3093c12a1/MR_Logo_LIBSYN_For_APple.jpg","title":"The Majority Report with Sam Seder","url":"https://static.libsyn.com/p/assets/1/2/7/5/1275caa5c11b3ea55f2e77a3093c12a1/MR_Logo_LIBSYN_For_APple.jpg","width":null},"title":"The Majority Report with Sam Seder"},"stream_id":"34c3c08d-0d55-4af4-a126-170c1380f07e","url":"https://feeds.feedburner.com/MajorityReport"}}]),
            events_count: 1,
        },
        CommitAttempt {
            stream_metadata: StreamMetadata {
                stream_id: uuid!["34c3c08d-0d55-4af4-a126-170c1380f07e"],
                stream_version: 1,
            },
            commit_metadata: CommitMetadata {
                commit_id: uuid!["929e10d1-9159-478a-8baf-1c461a539fc0"],
                commit_sequence: 1,
                commit_timestamp: Utc::now(),
            },
            metadata: json!({"AddFeed":{"feed_id":"6a278a5d-8f37-4574-9622-c74585d02eb4","stream_id":"34c3c08d-0d55-4af4-a126-170c1380f07e","url":"http://feeds.feedburner.com/dancarlin/history?format=xml"}}),
            events: json!([{"Added":{"entries":[],"feed_id":"6a278a5d-8f37-4574-9622-c74585d02eb4","metadata":{"authors":["Dan Carlin"],"description":"In \"Hardcore History\" journalist and broadcaster Dan Carlin takes his \"Martian\", unorthodox way of thinking and applies it to the past. Was Alexander the Great as bad a person as Adolf Hitler? What would Apaches with modern weapons be like? Will our modern civilization ever fall like civilizations from past eras? This isn't academic history (and Carlin isn't a historian) but the podcast's unique blend of high drama, masterful narration and Twilight Zone-style twists has entertained millions of listeners.","logo":{"description":null,"height":null,"link":{"href":"http://www.dancarlin.com/","href_lang":null,"length":null,"media_type":null,"rel":null,"title":null},"raw_url":"http://www.dancarlin.com/graphics/DC_HH_iTunes.jpg","title":"Dan Carlin's Hardcore History","url":"http://www.dancarlin.com/graphics/DC_HH_iTunes.jpg","width":null},"title":"Dan Carlin's Hardcore History"},"stream_id":"34c3c08d-0d55-4af4-a126-170c1380f07e","url":"http://feeds.feedburner.com/dancarlin/history?format=xml"}}]),
            events_count: 1,
        },
        CommitAttempt {
            stream_metadata: StreamMetadata {
                stream_id: uuid!["34c3c08d-0d55-4af4-a126-170c1380f07e"],
                stream_version: 2,
            },
            commit_metadata: CommitMetadata {
                commit_id: uuid!["5b6ec1c2-078d-4f1c-a09b-d8c59d2c71b0"],
                commit_sequence: 2,
                commit_timestamp: Utc::now(),
            },
            metadata: json!({"AddFeed":{"feed_id":"47c2f2d3-86d9-4065-8745-9b88fd877148","stream_id":"34c3c08d-0d55-4af4-a126-170c1380f07e","url":"https://historyofrome.libsyn.com/rss"}}),
            events: json!([{"Added":{"entries":[],"feed_id":"47c2f2d3-86d9-4065-8745-9b88fd877148","metadata":{"authors":["Mike Duncan"],"description":"A weekly podcast tracing the history of the Roman Empire, beginning with Aeneas's arrival in Italy and ending with the exile of Romulus Augustulus, last Emperor of the Western Roman Empire. Now complete!","logo":{"description":null,"height":null,"link":{"href":"http://thehistoryofrome.typepad.com/the_history_of_rome/","href_lang":null,"length":null,"media_type":null,"rel":null,"title":null},"raw_url":"https://static.libsyn.com/p/assets/1/2/f/c/12fc067020662e91/THoR_Logo_1500x1500.jpg","title":"The History of Rome","url":"https://static.libsyn.com/p/assets/1/2/f/c/12fc067020662e91/THoR_Logo_1500x1500.jpg","width":null},"title":"The History of Rome"},"stream_id":"34c3c08d-0d55-4af4-a126-170c1380f07e","url":"https://historyofrome.libsyn.com/rss"}}]),
            events_count: 1,
        },
    ];

    let mut aggregate = FeedAggregate::default();
    let commit_store_client = CommitStoreClient::connect("http://localhost:7392")
        .await
        .unwrap();
    let mut client = Client::new(commit_store_client);
    for commit_attempt in commit_attempts {
        let commit_result = client.commit(&commit_attempt).await.unwrap();
        let events: Vec<FeedAggregateEvent> = serde_json::from_value(commit_result.events).unwrap();
        aggregate = aggregate.apply_many(events);
    }
    println!("ok");
}
