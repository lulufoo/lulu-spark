//! Net kernel tests: pure plan resolution plus observable-proxy routing.
//!
//! Env vars are process-global, and every plan build reads them, so all net
//! tests serialize on one lock and clean the proxy vars afterwards.

mod async_side;
mod business_paths;
pub(crate) mod kernel;
pub(crate) mod plan;

use std::sync::{Arc, Mutex, MutexGuard};

use super::Sources;
use crate::host::{ProxyEndpoint, SystemProxySnapshot, SystemProxySource};

pub(crate) static TEST_LOCK: Mutex<()> = Mutex::new(());

pub(crate) fn lock_env() -> MutexGuard<'static, ()> {
    TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

pub(crate) const PROXY_ENV_VARS: &[&str] = &[
    "HTTP_PROXY",
    "http_proxy",
    "HTTPS_PROXY",
    "https_proxy",
    "ALL_PROXY",
    "all_proxy",
    "NO_PROXY",
    "no_proxy",
    "REQUEST_METHOD",
];

/// Removes the proxy vars on drop, so a panicked test cannot poison the rest.
pub(crate) struct EnvCleaner;

impl Drop for EnvCleaner {
    fn drop(&mut self) {
        for name in PROXY_ENV_VARS {
            std::env::remove_var(name);
        }
    }
}

pub(crate) fn set_env(name: &str, value: &str) {
    std::env::set_var(name, value);
}

pub(crate) struct FakeSource(SystemProxySnapshot);

impl SystemProxySource for FakeSource {
    fn snapshot(&self) -> SystemProxySnapshot {
        self.0.clone()
    }
}

/// A source the test mutates in place — the stand-in for macOS system proxy
/// changes observed by the watcher.
pub(crate) struct DynamicSource(pub Arc<Mutex<SystemProxySnapshot>>);

impl SystemProxySource for DynamicSource {
    fn snapshot(&self) -> SystemProxySnapshot {
        self.0.lock().expect("dynamic source").clone()
    }
}

pub(crate) fn fake_source(snapshot: SystemProxySnapshot) -> Arc<dyn SystemProxySource> {
    Arc::new(FakeSource(snapshot))
}

pub(crate) fn snapshot(
    http: Option<(&str, Option<u16>)>,
    https: Option<(&str, Option<u16>)>,
    exceptions: &[&str],
) -> SystemProxySnapshot {
    let endpoint = |entry: Option<(&str, Option<u16>)>| {
        entry.map(|(host, port)| ProxyEndpoint {
            host: host.to_string(),
            port,
        })
    };
    SystemProxySnapshot {
        http: endpoint(http),
        https: endpoint(https),
        exceptions: exceptions.iter().map(|item| item.to_string()).collect(),
    }
}

/// A kernel no one else swaps: direct plan, fixed sources. Rigs pin one so
/// their connection-reuse assertions stay independent of the static kernel.
pub(crate) fn pinned_kernel() -> Arc<super::Kernel> {
    Arc::new(kernel_with(false, snapshot(None, None, &[])))
}

pub(crate) fn kernel_with(use_system_proxy: bool, system: SystemProxySnapshot) -> super::Kernel {
    let toggle: Arc<dyn Fn() -> bool + Send + Sync> = Arc::new(move || use_system_proxy);
    let sources = Sources {
        toggle,
        system: fake_source(system),
    };
    super::Kernel::with_sources(sources)
}
