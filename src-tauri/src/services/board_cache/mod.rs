//! Board reads, durable access invalidation, and in-process request generations.

use std::sync::{Arc, Condvar, Mutex, OnceLock};

use serde::Serialize;

use crate::config::vault::BoardCredentialSnapshot;
use crate::repositories::board_cache::{BoardCacheRecord, StoreOutcome};
use super::board_sketch::{failure, BoardDocument, BoardReadIdentity, BoardSketchReply, DocumentFetch};

mod backend;
mod local;
mod remote;
mod state;

use state::State;

pub(crate) trait CredentialStore: Send + Sync {
    fn get(&self) -> Result<Option<BoardCredentialSnapshot>, String>;
    fn set(&self, value: &str) -> Result<(), String>;
    fn delete(&self) -> Result<(), String>;
    fn revoke(&self, scope: &str) -> Result<bool, String>;
    fn deny(&self, scope: &str, diagram_id: &str, denied: bool) -> Result<bool, String>;
}

pub(crate) trait SourceCache: Send + Sync {
    fn read(&self, url: &str, scope: &str) -> Result<Option<BoardCacheRecord>, String>;
    fn store(&self, record: &BoardCacheRecord) -> Result<StoreOutcome, String>;
    fn remove_url(&self, url: &str) -> Result<(), String>;
    fn remove_diagram(&self, id: &str, scope: &str) -> Result<usize, String>;
    fn remove_scope(&self, scope: &str) -> Result<usize, String>;
    fn clear(&self) -> Result<(), String>;
}

pub(crate) trait Fetcher: Send + Sync {
    fn fetch(&self, credential: &str, diagram_id: &str) -> DocumentFetch;
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct BoardCacheInvalidated {
    pub credential_scope: String,
    pub generation: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagram_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagram_generation: Option<u64>,
    pub reason: &'static str,
}

pub type InvalidatedHandler = Arc<dyn Fn(BoardCacheInvalidated) + Send + Sync>;

pub(crate) struct Coordinator {
    credentials: Arc<dyn CredentialStore>,
    cache: Arc<dyn SourceCache>,
    fetcher: Arc<dyn Fetcher>,
    state: Mutex<State>,
    handler: Mutex<Option<InvalidatedHandler>>,
}

impl Coordinator {
    pub(crate) fn new(
        credentials: Arc<dyn CredentialStore>,
        cache: Arc<dyn SourceCache>,
        fetcher: Arc<dyn Fetcher>,
    ) -> Self {
        Self {
            credentials, cache, fetcher,
            state: Mutex::new(State::default()),
            handler: Mutex::new(None),
        }
    }

    fn emit(&self, events: Vec<BoardCacheInvalidated>) {
        let handler = self.handler.lock().unwrap_or_else(|e| e.into_inner()).clone();
        if let Some(handler) = handler {
            for event in events { handler(event); }
        }
    }

    fn record_cache_error<T>(&self, action: &'static str, result: Result<T, String>) -> Option<T> {
        match result {
            Ok(value) => Some(value),
            Err(_) => {
                super::board_sketch_log::cache_failure(action);
                None
            }
        }
    }
}

#[derive(Clone)]
struct CompletedRead {
    result: Result<BoardDocument, BoardSketchReply>,
    identity: BoardReadIdentity,
    bytes: Option<usize>,
}

struct Flight {
    identity: BoardReadIdentity,
    completed: Mutex<Option<CompletedRead>>,
    changed: Condvar,
}

impl Flight {
    fn new(identity: BoardReadIdentity) -> Self {
        Self { identity, completed: Mutex::new(None), changed: Condvar::new() }
    }

    fn complete(&self, value: CompletedRead) {
        let mut completed = self.completed.lock().unwrap_or_else(|e| e.into_inner());
        if completed.is_none() {
            *completed = Some(value);
            self.changed.notify_all();
        }
    }

    fn cancel(&self) {
        self.complete(CompletedRead {
            result: Err(failure("superseded")),
            identity: self.identity.clone(), bytes: None,
        });
    }

    fn wait(&self) -> CompletedRead {
        let mut value = self.completed.lock().unwrap_or_else(|e| e.into_inner());
        while value.is_none() {
            value = self.changed.wait(value).unwrap_or_else(|e| e.into_inner());
        }
        value.as_ref().expect("completed flight").clone()
    }
}

fn attach(mut reply: BoardSketchReply, identity: BoardReadIdentity) -> BoardSketchReply {
    reply.identity = Some(identity);
    reply
}

fn get_coordinator() -> &'static Coordinator {
    static COORDINATOR: OnceLock<Coordinator> = OnceLock::new();
    COORDINATOR.get_or_init(|| Coordinator::new(
        Arc::new(backend::VaultCredentials), Arc::new(backend::RuntimeCache),
        Arc::new(backend::HttpFetcher),
    ))
}

pub fn read_board_sketch(url: &str) -> BoardSketchReply { get_coordinator().read_remote(url) }
pub fn read_cached_board_sketch(url: &str) -> BoardSketchReply { get_coordinator().read_cached(url) }
pub fn set_board_pat(value: &str) -> Result<(), String> { get_coordinator().set_pat(value) }
pub fn delete_board_pat() -> Result<(), String> { get_coordinator().delete_pat() }

pub fn set_invalidated_handler(handler: Option<InvalidatedHandler>) {
    *get_coordinator().handler.lock().unwrap_or_else(|e| e.into_inner()) = handler;
}

#[cfg(test)]
#[path = "../../unit-tests/services/board_cache/mod.rs"]
mod tests;
