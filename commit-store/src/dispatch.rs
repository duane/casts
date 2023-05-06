// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tokio::sync::mpsc::{channel, Receiver, Sender};

use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    commit::{Commit, CommitAttempt},
    AsyncClientStore, StoreError,
};

#[derive(Clone)]
struct Subscriber {
    tx: Sender<Commit>,
    // commit_number: i64,
}

#[derive(Clone)]
pub struct Dispatcher {
    subscriptions: Arc<Mutex<HashMap<Uuid, Vec<Subscriber>>>>,
}

impl Dispatcher {
    pub fn new() -> Dispatcher {
        Dispatcher {
            subscriptions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn subscribe(&self, stream_id: Uuid) -> Receiver<Commit> {
        let (tx, rx) = channel(100);
        let mut subscriptions = self.subscriptions.lock().unwrap();
        match subscriptions.get_mut(&stream_id) {
            Some(list) => {
                list.push(Subscriber {
                    tx,
                    // commit_number: 0,
                });
            }
            None => {
                _ = subscriptions.insert(
                    stream_id,
                    vec![Subscriber {
                        tx,
                        // commit_number: 0,
                    }],
                );
            }
        }
        rx
    }

    pub async fn dispatch(&self, commit: Commit) {
        let subscriptions = match self.subscriptions.lock() {
            Ok(unlocked) => unlocked.get(&commit.stream_id).map(|v| v.to_owned()),
            Err(_) => panic!(),
        };
        match subscriptions {
            Some(subs) => {
                for sub in subs {
                    sub.tx.send(commit.clone()).await.unwrap()
                }
            }
            None => (),
        }
    }
}

#[derive(Clone)]
pub struct DispatchedStore<S> {
    store: S,
    dispatcher: Dispatcher,
}

impl<S> DispatchedStore<S> {
    pub fn new(s: S) -> DispatchedStore<S> {
        DispatchedStore {
            store: s,
            dispatcher: Dispatcher::new(),
        }
    }

    pub fn subscribe(&self, stream_id: &Uuid) -> Receiver<Commit> {
        self.dispatcher.subscribe(stream_id.clone())
    }
}

#[async_trait]
impl<S: AsyncClientStore> AsyncClientStore for DispatchedStore<S> {
    async fn commit(
        &mut self,
        commit_attempt: &CommitAttempt,
    ) -> Result<Commit, Box<dyn StoreError>> {
        let commit = self.store.commit(&commit_attempt).await?;
        self.dispatcher.dispatch(commit.clone()).await;
        Ok(commit)
    }

    async fn get_stream_range(
        &mut self,
        stream_id: Uuid,
        min_version: i64,
        max_version: i64,
    ) -> Result<Vec<Commit>, Box<dyn StoreError>> {
        self.store
            .get_stream_range(stream_id, min_version, max_version)
            .await
    }

    async fn get_range(
        &mut self,
        min_commit_number: i64,
        max_commit_number: i64,
    ) -> Result<Vec<Commit>, Box<dyn StoreError>> {
        self.store
            .get_range(min_commit_number, max_commit_number)
            .await
    }

    async fn get_commit(&mut self, commit_id: &Uuid) -> Result<Commit, Box<dyn StoreError>> {
        self.store.get_commit(commit_id).await
    }
}
