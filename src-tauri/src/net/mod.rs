//! Unified network kernel: proxy strategy, client lifecycle, hot switch.
//!
//! Cross-cutting infrastructure, not a layer: L0 Gateway, L1 MCP Host / Main
//! Host, and L4 Agent / Services call it; it only reads host facts downward
//! and reaches the outside network. Behavior contract: the Linear document
//! "LW-809 统一网络模块与系统代理：行为与接口契约".

pub mod plan;
pub mod spec;

pub mod async_side;

use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::time::Duration;

use reqwest::blocking::Client;
use reqwest::redirect::Policy;
use reqwest::{NoProxy, Proxy};
use serde_json::json;

use crate::host::{production_source, SystemProxySnapshot, SystemProxySource};
use crate::services::app_log::{self, Level, Side};

pub use plan::{PlanInputs, ProxyPlan, ProxySource, ResolvedProxy};
pub use spec::{ClientSpec, RedirectSpec};

/// Where the Network toggle (`network.use_system_proxy`) reads from. The
/// production source is installed at boot; until then the toggle reads off.
pub type ToggleSource = Arc<dyn Fn() -> bool + Send + Sync>;

#[derive(Debug)]
pub enum NetError {
    /// The client (or its proxy entries) could not be built.
    ClientBuild(String),
}

impl std::fmt::Display for NetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NetError::ClientBuild(text) => write!(f, "client build: {text}"),
        }
    }
}

struct Sources {
    toggle: ToggleSource,
    system: Arc<dyn SystemProxySource>,
}

/// One generation of the plan plus the clients built under it. A request
/// that obtained a client from here keeps working after a swap: its `Arc`
/// holds the generation alive until the last reference drops.
struct Generation {
    number: u64,
    plan: ProxyPlan,
    warnings: Vec<String>,
    clients: Mutex<HashMap<ClientSpec, Arc<Client>>>,
}

pub struct Kernel {
    sources: RwLock<Sources>,
    state: RwLock<Arc<Generation>>,
}

static KERNEL: std::sync::LazyLock<Kernel> = std::sync::LazyLock::new(Kernel::production);

fn sources() -> Sources {
    Sources {
        toggle: Arc::new(|| false),
        system: production_source(),
    }
}

impl Kernel {
    /// Production kernel: toggle off until boot installs the settings-backed
    /// source, system proxy read from SCDynamicStore.
    pub fn production() -> Self {
        Self::with_sources(sources())
    }

    pub(crate) fn with_sources(sources: Sources) -> Self {
        let kernel = Self {
            sources: RwLock::new(Sources {
                toggle: sources.toggle,
                system: sources.system,
            }),
            state: RwLock::new(Arc::new(Generation {
                number: 1,
                plan: ProxyPlan::direct(),
                warnings: Vec::new(),
                clients: Mutex::new(HashMap::new()),
            })),
        };
        kernel.refresh();
        kernel
    }

    fn generation(&self) -> Arc<Generation> {
        self.state.read().unwrap_or_else(|e| e.into_inner()).clone()
    }

    fn build_generation(&self, number: u64) -> Generation {
        let sources = self.sources.read().unwrap_or_else(|e| e.into_inner());
        let use_system_proxy = (sources.toggle)();
        let system = if use_system_proxy {
            sources.system.snapshot()
        } else {
            SystemProxySnapshot::default()
        };
        drop(sources);
        let resolved = plan::resolve(&PlanInputs { use_system_proxy, system });
        Generation {
            number,
            plan: resolved.plan,
            warnings: resolved.warnings,
            clients: Mutex::new(HashMap::new()),
        }
    }

    /// Rebuild the plan from the current sources. `true` when the plan
    /// changed and the next acquisition starts a new generation; already
    /// acquired clients keep working (D5). Unchanged plans stay silent — the
    /// watcher rebuilds on a fixed interval and must not spam the log.
    pub fn refresh(&self) -> bool {
        let next = self.generation().number + 1;
        let generation = self.build_generation(next);
        let mut state = self.state.write().unwrap_or_else(|e| e.into_inner());
        if generation.plan == state.plan {
            return false;
        }
        app_log::log(
            "net",
            "plan_changed",
            None,
            Some(json!({
                "generation": generation.number,
                "plan": plan::summary_json(&generation.plan),
                "warnings": generation.warnings,
            })),
            Side::Host,
            Level::Info,
        );
        *state = Arc::new(generation);
        true
    }

    /// The current-generation client for `spec`. Acquisition happens at
    /// request-build time; the returned `Arc` pins the generation for as long
    /// as the request holds it (LW-809 D5).
    pub fn external(&self, spec: &ClientSpec) -> Result<Arc<Client>, NetError> {
        let generation = self.generation();
        let mut clients = generation.clients.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(client) = clients.get(spec) {
            return Ok(Arc::clone(client));
        }
        let client = Arc::new(build_blocking_client(&generation.plan, spec)?);
        clients.insert(spec.clone(), Arc::clone(&client));
        Ok(client)
    }

    /// A loopback client: always direct, never taken over by the system or
    /// environment proxy (LW-809 D2).
    pub fn loopback(&self, request_timeout: Duration) -> Result<Client, NetError> {
        Client::builder()
            .timeout(request_timeout)
            .no_proxy()
            .build()
            .map_err(|e| NetError::ClientBuild(e.to_string()))
    }

    /// Replace the sources, then rebuild. Part of boot wiring (LW-811).
    pub fn install(&self, toggle: ToggleSource, system: Arc<dyn SystemProxySource>) -> bool {
        *self.sources.write().unwrap_or_else(|e| e.into_inner()) = Sources { toggle, system };
        self.refresh()
    }

    /// Credentials-free summary of the current generation.
    pub fn snapshot_json(&self) -> serde_json::Value {
        let generation = self.generation();
        json!({
            "generation": generation.number,
            "plan": plan::summary_json(&generation.plan),
        })
    }

    /// A reqwest 0.13 client from the current plan — the MCP transport's
    /// entry. Snapshot at call time; no caching (one per connect).
    pub fn async_external(&self, spec: &ClientSpec) -> Result<reqwest013::Client, NetError> {
        let plan = self.current_plan();
        async_side::build_client(&plan, spec)
    }

    /// The plan as of now; the async side snapshots it at connect time.
    pub(crate) fn current_plan(&self) -> ProxyPlan {
        self.generation().plan.clone()
    }
}

pub fn kernel() -> &'static Kernel {
    &KERNEL
}

/// The current-generation client for `spec`, from the process kernel.
pub fn external(spec: &ClientSpec) -> Result<Arc<Client>, NetError> {
    KERNEL.external(spec)
}

/// A loopback client from the process kernel.
pub fn loopback(request_timeout: Duration) -> Result<Client, NetError> {
    KERNEL.loopback(request_timeout)
}

/// Rebuild the plan now (the settings-save hook).
pub fn refresh_now() -> bool {
    KERNEL.refresh()
}

/// Point the process kernel at the persisted toggle and the real system
/// source (boot wiring).
pub fn install(toggle: ToggleSource, system: Arc<dyn SystemProxySource>) -> bool {
    KERNEL.install(toggle, system)
}

/// Poll the sources on a fixed interval so macOS system proxy changes reach
/// new requests without a restart (LW-809 D4). Unchanged plans stay silent.
pub fn spawn_watcher(interval: Duration) {
    let _ = std::thread::Builder::new()
        .name("spark-net-watch".into())
        .spawn(move || loop {
            std::thread::sleep(interval);
            refresh_now();
        });
}

/// Credentials-free snapshot of the current generation (`get_config`).
pub fn snapshot_json() -> serde_json::Value {
    KERNEL.snapshot_json()
}

fn build_blocking_client(plan: &ProxyPlan, spec: &ClientSpec) -> Result<Client, NetError> {
    // `no_proxy()` first: the plan is authoritative and env vars must not be
    // re-read behind its back (D7).
    let mut builder = Client::builder().no_proxy();
    if let Some(timeout) = spec.connect_timeout {
        builder = builder.connect_timeout(timeout);
    }
    if let Some(timeout) = spec.request_timeout {
        builder = builder.timeout(timeout);
    }
    if let Some(timeout) = spec.pool_idle_timeout {
        builder = builder.pool_idle_timeout(timeout);
    }
    if let Some(max) = spec.pool_max_idle_per_host {
        builder = builder.pool_max_idle_per_host(max as usize);
    }
    builder = match spec.redirect {
        RedirectSpec::None => builder.redirect(Policy::none()),
        RedirectSpec::Default => builder,
    };
    builder = apply_plan(builder, plan)?;
    builder.build().map_err(|e| NetError::ClientBuild(e.to_string()))
}

fn apply_plan(builder: reqwest::blocking::ClientBuilder, plan: &ProxyPlan) -> Result<reqwest::blocking::ClientBuilder, NetError> {
    if plan.http.is_none() && plan.https.is_none() {
        return Ok(builder);
    }
    let no_proxy = || {
        NoProxy::from_string(&plan.no_proxy)
            .expect("no_proxy list is never empty: loopback entries are implicit")
    };
    let mut builder = builder;
    if let Some(proxy) = &plan.http {
        let proxy = Proxy::http(&proxy.url)
            .map_err(|e| NetError::ClientBuild(format!("http proxy: {e}")))?
            .no_proxy(Some(no_proxy()));
        builder = builder.proxy(proxy);
    }
    if let Some(proxy) = &plan.https {
        let proxy = Proxy::https(&proxy.url)
            .map_err(|e| NetError::ClientBuild(format!("https proxy: {e}")))?
            .no_proxy(Some(no_proxy()));
        builder = builder.proxy(proxy);
    }
    Ok(builder)
}

#[cfg(test)]
#[path = "../unit-tests/net/mod.rs"]
pub(crate) mod tests;
