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
    AsyncClientStore, CommitStream, DispatchingAsyncClientStore, StoreError,
};

#[async_trait]
pub trait Dispatcher {
    async fn dispatch(&self, commit: Commit);
}

#[derive(Clone, Debug)]
struct LocalSubscriber {
    tx: Sender<Commit>,
    // commit_number: i64,
}

#[derive(Clone, Debug)]
pub struct LocalDispatcher {
    subscriptions: Arc<Mutex<HashMap<Uuid, Vec<LocalSubscriber>>>>,
}

impl LocalDispatcher {
    pub fn new() -> LocalDispatcher {
        LocalDispatcher {
            subscriptions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn subscribe(&self, stream_id: Uuid) -> Receiver<Commit> {
        let (tx, rx) = channel(100);
        let mut subscriptions = self.subscriptions.lock().unwrap();
        match subscriptions.get_mut(&stream_id) {
            Some(list) => {
                list.push(LocalSubscriber {
                    tx,
                    // commit_number: 0,
                });
            }
            None => {
                _ = subscriptions.insert(
                    stream_id,
                    vec![LocalSubscriber {
                        tx,
                        // commit_number: 0,
                    }],
                );
            }
        }
        rx
    }
}

#[derive(Clone, Debug)]
pub struct DispatchedStore<S: AsyncClientStore + Send> {
    store: S,
    dispatcher: LocalDispatcher,
}

#[async_trait]
impl<S: AsyncClientStore + Send> DispatchingAsyncClientStore for DispatchedStore<S> {
    async fn subscribe(&mut self, stream_id: &Uuid) -> CommitStream {
        CommitStream::ReceiverBacked(self.dispatcher.subscribe(stream_id.to_owned()).into())
    }
}

impl<S: AsyncClientStore + Send> DispatchedStore<S> {
    pub fn new(s: S) -> DispatchedStore<S> {
        DispatchedStore {
            store: s,
            dispatcher: LocalDispatcher::new(),
        }
    }

    pub fn subscribe(&self, stream_id: &Uuid) -> Receiver<Commit> {
        self.dispatcher.subscribe(stream_id.clone())
    }
}

#[async_trait]
impl<S: AsyncClientStore + Send> AsyncClientStore for DispatchedStore<S> {
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

#[async_trait]
impl Dispatcher for LocalDispatcher {
    async fn dispatch(&self, commit: Commit) {
        let subscribers = {
            if let Some(subscribers) = self.subscriptions.lock().unwrap().get(&commit.stream_id) {
                subscribers.clone()
            } else {
                return;
            }
        };
        for subscriber in subscribers {
            let commit = commit.clone();
            let _ = subscriber.tx.send(commit).await;
        }
    }
}
