//! Global `CursorAgentProcessManager` — Host-side 0..1 `cursor-agent-runner` lifecycle.
//!
//! Public access surface: [`CursorAgentProcessManager::warm`],
//! [`CursorAgentProcessManager::ensure_client`], [`CursorAgentProcessManager::shutdown`].
//! `ProcessEntry` generation is internal (replacement / stale-access marking only).

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use crate::config::paths;
use crate::config::settings;
use serde_json::{json, Value};

use crate::services::agent::cursor_adapter::{
    CursorError, CursorRunnerClient, ProcessCursorRunnerClient,
};
use crate::services::agent::engine_router::{self, EngineKind, EngineRuntimeConfig};

type ConfigFn = Box<dyn Fn() -> Result<EngineRuntimeConfig, String> + Send>;
type SpawnFn =
    Box<dyn FnMut(&str) -> Result<Box<dyn CursorRunnerClient>, CursorError> + Send>;

/// Formal process-layer lifecycle (tech-doc leaf-process-manager-AR).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProcessLifecycleState {
    Absent,
    Starting,
    Ready,
    Invalid,
    Replacing,
    Stopping,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WarmError {
    Spawn,
    Config(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EnsureError {
    Unavailable,
    Spawn,
    Stale,
    Config(String),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShutdownError {
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HostCloseError {
    InvalidBusinessId,
    Unavailable,
    Spawn,
    Stale,
    Config(String),
    Runner(CursorError),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RequestError {
    Stale,
    Runner(CursorError),
}

struct ProcessEntry {
    generation: u64,
    api_key_fingerprint: String,
    client: Arc<Mutex<Box<dyn CursorRunnerClient>>>,
}

struct Inner {
    state: ProcessLifecycleState,
    entry: Option<ProcessEntry>,
}

/// Temporary access to the managed runner client. Invalid after generation change
/// or manager invalidate/shutdown.
pub struct ClientAccess {
    client: Arc<Mutex<Box<dyn CursorRunnerClient>>>,
    generation: u64,
    current_gen: Arc<AtomicU64>,
}

impl std::fmt::Debug for ClientAccess {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClientAccess")
            .field("generation", &self.generation)
            .finish_non_exhaustive()
    }
}

impl ClientAccess {
    /// Process generation this access was issued for (foreground bind / stale checks).
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// Submit JSONL request with caller-provided `request_id` (not generated here).
    /// Signature has no `session_id` — client must not serialize on session.
    /// Prefer concurrent JSONL handle (pending-map demux) so the client mutex is
    /// not held across the response wait.
    pub fn request(
        &self,
        request_id: &str,
        method: &str,
        params: Option<Value>,
    ) -> Result<Value, RequestError> {
        if self.current_gen.load(Ordering::SeqCst) != self.generation {
            return Err(RequestError::Stale);
        }
        let concurrent = {
            let guard = self.client.lock().unwrap_or_else(|e| e.into_inner());
            if self.current_gen.load(Ordering::SeqCst) != self.generation {
                return Err(RequestError::Stale);
            }
            guard.concurrent_jsonl()
        };
        if let Some(handle) = concurrent {
            if self.current_gen.load(Ordering::SeqCst) != self.generation {
                return Err(RequestError::Stale);
            }
            return handle
                .submit(request_id, method, params)
                .map_err(RequestError::Runner);
        }
        let mut guard = self.client.lock().unwrap_or_else(|e| e.into_inner());
        if self.current_gen.load(Ordering::SeqCst) != self.generation {
            return Err(RequestError::Stale);
        }
        guard
            .request(request_id, method, params)
            .map_err(RequestError::Runner)
    }
}

/// Unique Host owner of 0..1 managed `ProcessCursorRunnerClient` / runner process.
pub struct CursorAgentProcessManager {
    inner: Mutex<Inner>,
    config: Mutex<ConfigFn>,
    spawn: Mutex<SpawnFn>,
    current_gen: Arc<AtomicU64>,
    next_generation: AtomicU64,
    next_request_id: AtomicU64,
}

static GLOBAL: OnceLock<CursorAgentProcessManager> = OnceLock::new();

impl CursorAgentProcessManager {
    pub fn global() -> &'static CursorAgentProcessManager {
        GLOBAL.get_or_init(Self::production)
    }

    pub fn production() -> Self {
        let repo = paths::repo_root().unwrap_or_else(|_| PathBuf::from("."));
        Self::with_deps(
            || {
                let s = settings::load().map_err(|e| e.to_string())?;
                engine_router::read_engine_runtime_config(&s).map_err(|e| e.to_string())
            },
            move |api_key| {
                let mut client = ProcessCursorRunnerClient::spawn(&repo, api_key)?;
                client.on_spawned_with_api_key(api_key);
                Ok(Box::new(client) as Box<dyn CursorRunnerClient>)
            },
        )
    }

    pub fn with_deps_for_tests<C, F>(config: C, factory: F) -> Self
    where
        C: Fn() -> Result<EngineRuntimeConfig, String> + Send + 'static,
        F: FnMut(&str) -> Result<Box<dyn CursorRunnerClient>, CursorError> + Send + 'static,
    {
        Self::with_deps(config, factory)
    }

    fn with_deps<C, F>(config: C, factory: F) -> Self
    where
        C: Fn() -> Result<EngineRuntimeConfig, String> + Send + 'static,
        F: FnMut(&str) -> Result<Box<dyn CursorRunnerClient>, CursorError> + Send + 'static,
    {
        Self {
            inner: Mutex::new(Inner {
                state: ProcessLifecycleState::Absent,
                entry: None,
            }),
            config: Mutex::new(Box::new(config)),
            spawn: Mutex::new(Box::new(factory)),
            current_gen: Arc::new(AtomicU64::new(0)),
            next_generation: AtomicU64::new(1),
            next_request_id: AtomicU64::new(1),
        }
    }

    /// Host startup warm: spawn process + client only (no SDK Agent create).
    pub fn warm(&self) -> Result<(), WarmError> {
        let cfg = self.read_config().map_err(WarmError::Config)?;
        let Some(api_key) = cursor_credential(&cfg) else {
            return Ok(());
        };
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if inner.state == ProcessLifecycleState::Ready && inner.entry.is_some() {
            return Ok(());
        }
        self.start_locked(&mut inner, &api_key)
            .map_err(|e| match e {
                EnsureError::Spawn => WarmError::Spawn,
                EnsureError::Config(m) => WarmError::Config(m),
                _ => WarmError::Spawn,
            })
    }

    /// Return temporary access to the managed client; lazy-start / replace as needed.
    pub fn ensure_client(&self) -> Result<ClientAccess, EnsureError> {
        let cfg = self.read_config().map_err(EnsureError::Config)?;
        let Some(api_key) = cursor_credential(&cfg) else {
            return Err(EnsureError::Unavailable);
        };
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());

        let fp = fingerprint_api_key(&api_key);
        if let Some(entry) = inner.entry.as_ref() {
            if inner.state == ProcessLifecycleState::Ready && entry.api_key_fingerprint == fp {
                return Ok(self.access_for(entry));
            }
            if inner.state == ProcessLifecycleState::Ready && entry.api_key_fingerprint != fp {
                inner.state = ProcessLifecycleState::Replacing;
                self.clear_entry_locked(&mut inner);
            }
        }

        self.start_locked(&mut inner, &api_key)?;
        let entry = inner
            .entry
            .as_ref()
            .ok_or(EnsureError::Spawn)?;
        Ok(self.access_for(entry))
    }

    /// App-exit reclaim of client + child process.
    pub fn shutdown(&self) -> Result<(), ShutdownError> {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        inner.state = ProcessLifecycleState::Stopping;
        self.clear_entry_locked(&mut inner);
        inner.state = ProcessLifecycleState::Absent;
        Ok(())
    }

    /// Host-only operational/reset path: dispose one Runner Agent slot by its
    /// stable business identity while keeping the shared Runner process alive.
    pub fn close_agent_slot_by_business_id(
        &self,
        business_id: &str,
    ) -> Result<(), HostCloseError> {
        let business_id = business_id.trim();
        if business_id.is_empty() {
            return Err(HostCloseError::InvalidBusinessId);
        }

        let access = self.ensure_client().map_err(HostCloseError::from)?;
        let request_id = format!(
            "host-close-{}",
            self.next_request_id.fetch_add(1, Ordering::SeqCst)
        );
        match access.request(
            &request_id,
            "close",
            Some(json!({ "business_id": business_id })),
        ) {
            Ok(_) => Ok(()),
            Err(RequestError::Stale) => Err(HostCloseError::Stale),
            Err(RequestError::Runner(error)) => {
                self.invalidate();
                Err(HostCloseError::Runner(error))
            }
        }
    }

    pub fn reset_global_for_tests() {
        if let Some(mgr) = GLOBAL.get() {
            mgr.reset_instance_for_tests();
        }
    }

    pub fn lifecycle_state_for_tests(&self) -> ProcessLifecycleState {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .state
    }

    pub fn managed_runner_count_for_tests(&self) -> usize {
        usize::from(
            self.inner
                .lock()
                .unwrap_or_else(|e| e.into_inner())
                .entry
                .is_some(),
        )
    }

    pub fn generation_for_tests(&self) -> u64 {
        self.inner
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .entry
            .as_ref()
            .map(|e| e.generation)
            .unwrap_or(0)
    }

    /// Production invalidate: process/JSONL failure path.
    /// `Ready|Replacing|Starting` → briefly `Invalid` → clear entry (bump generation) → `Absent`.
    /// Next [`Self::ensure_client`] lazy-rebuilds; the failed request is not replayed.
    pub fn invalidate(&self) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        match inner.state {
            ProcessLifecycleState::Ready
            | ProcessLifecycleState::Replacing
            | ProcessLifecycleState::Starting => {
                inner.state = ProcessLifecycleState::Invalid;
                self.clear_entry_locked(&mut inner);
                inner.state = ProcessLifecycleState::Absent;
            }
            ProcessLifecycleState::Invalid
            | ProcessLifecycleState::Absent
            | ProcessLifecycleState::Stopping => {}
        }
    }

    fn reset_instance_for_tests(&self) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        self.clear_entry_locked(&mut inner);
        inner.state = ProcessLifecycleState::Absent;
        self.next_generation.store(1, Ordering::SeqCst);
        self.next_request_id.store(1, Ordering::SeqCst);
    }

    fn read_config(&self) -> Result<EngineRuntimeConfig, String> {
        let guard = self.config.lock().unwrap_or_else(|e| e.into_inner());
        (guard)()
    }

    fn access_for(&self, entry: &ProcessEntry) -> ClientAccess {
        ClientAccess {
            client: entry.client.clone(),
            generation: entry.generation,
            current_gen: self.current_gen.clone(),
        }
    }

    fn start_locked(
        &self,
        inner: &mut Inner,
        api_key: &str,
    ) -> Result<(), EnsureError> {
        inner.state = ProcessLifecycleState::Starting;
        let spawn_result = {
            let mut spawn = self.spawn.lock().unwrap_or_else(|e| e.into_inner());
            (spawn)(api_key)
        };
        match spawn_result {
            Ok(client) => {
                let generation = self.next_generation.fetch_add(1, Ordering::SeqCst);
                let client = Arc::new(Mutex::new(client));
                inner.entry = Some(ProcessEntry {
                    generation,
                    api_key_fingerprint: fingerprint_api_key(api_key),
                    client,
                });
                self.current_gen.store(generation, Ordering::SeqCst);
                inner.state = ProcessLifecycleState::Ready;
                Ok(())
            }
            Err(_) => {
                inner.entry = None;
                self.current_gen.store(0, Ordering::SeqCst);
                inner.state = ProcessLifecycleState::Absent;
                Err(EnsureError::Spawn)
            }
        }
    }

    fn clear_entry_locked(&self, inner: &mut Inner) {
        if let Some(entry) = inner.entry.take() {
            reclaim_client(entry.client);
        }
        self.current_gen.store(0, Ordering::SeqCst);
    }
}

/// Module-level alias used by tests (`process_manager::reset_global_for_tests`).
pub fn reset_global_for_tests() {
    CursorAgentProcessManager::reset_global_for_tests();
}

fn cursor_credential(cfg: &EngineRuntimeConfig) -> Option<String> {
    if cfg.engine != EngineKind::Cursor {
        return None;
    }
    cfg.credential
        .as_ref()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
}

/// Non-reversible fingerprint for ProcessEntry (never store raw API key).
fn fingerprint_api_key(api_key: &str) -> String {
    let mut hasher = DefaultHasher::new();
    "cursor-agent-api-key-fp-v1".hash(&mut hasher);
    api_key.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

#[cfg(test)]
pub fn fingerprint_api_key_for_tests(api_key: &str) -> String {
    fingerprint_api_key(api_key)
}

fn reclaim_client(client: Arc<Mutex<Box<dyn CursorRunnerClient>>>) {
    let mut guard = client.lock().unwrap_or_else(|e| e.into_inner());
    let _ = guard.close();
    guard.force_kill();
}

impl From<EnsureError> for HostCloseError {
    fn from(error: EnsureError) -> Self {
        match error {
            EnsureError::Unavailable => Self::Unavailable,
            EnsureError::Spawn => Self::Spawn,
            EnsureError::Stale => Self::Stale,
            EnsureError::Config(message) => Self::Config(message),
        }
    }
}
