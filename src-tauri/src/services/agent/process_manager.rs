//! Global `CursorAgentProcessManager` — Host-side 0..1 `cursor-agent-runner` lifecycle.
//!
//! Public access surface: [`CursorAgentProcessManager::warm`],
//! [`CursorAgentProcessManager::ensure_client`], [`CursorAgentProcessManager::shutdown`].
//! `ProcessEntry` generation is internal (replacement / stale-access marking only).

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};

use crate::config::paths;
use crate::config::settings;
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
    pub fn with_client<F, R>(&self, f: F) -> Result<R, EnsureError>
    where
        F: FnOnce(&mut dyn CursorRunnerClient) -> R,
    {
        if self.current_gen.load(Ordering::SeqCst) != self.generation {
            return Err(EnsureError::Stale);
        }
        let mut guard = self
            .client
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        if self.current_gen.load(Ordering::SeqCst) != self.generation {
            return Err(EnsureError::Stale);
        }
        Ok(f(guard.as_mut()))
    }
}

/// Unique Host owner of 0..1 managed `ProcessCursorRunnerClient` / runner process.
pub struct CursorAgentProcessManager {
    inner: Mutex<Inner>,
    config: Mutex<ConfigFn>,
    spawn: Mutex<SpawnFn>,
    current_gen: Arc<AtomicU64>,
    next_generation: AtomicU64,
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

        if let Some(entry) = inner.entry.as_ref() {
            if inner.state == ProcessLifecycleState::Ready
                && entry.api_key_fingerprint == api_key
            {
                return Ok(self.access_for(entry));
            }
            if inner.state == ProcessLifecycleState::Ready
                && entry.api_key_fingerprint != api_key
            {
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

    pub fn mark_invalid_for_tests(&self) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        if inner.state == ProcessLifecycleState::Ready {
            inner.state = ProcessLifecycleState::Invalid;
            self.clear_entry_locked(&mut inner);
            inner.state = ProcessLifecycleState::Absent;
        }
    }

    fn reset_instance_for_tests(&self) {
        let mut inner = self.inner.lock().unwrap_or_else(|e| e.into_inner());
        self.clear_entry_locked(&mut inner);
        inner.state = ProcessLifecycleState::Absent;
        self.next_generation.store(1, Ordering::SeqCst);
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
                    api_key_fingerprint: api_key.to_string(),
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

fn reclaim_client(client: Arc<Mutex<Box<dyn CursorRunnerClient>>>) {
    let mut guard = client.lock().unwrap_or_else(|e| e.into_inner());
    let _ = guard.close();
    guard.force_kill();
}
