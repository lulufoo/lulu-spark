//! macOS system HTTP/HTTPS proxy state: a host process fact, read on demand
//! and flowing downward to the network kernel (see the LW-809 contract).

use std::sync::Arc;

/// One system-configured proxy: `host`, optional `port`.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProxyEndpoint {
    pub host: String,
    pub port: Option<u16>,
}

/// What the system currently configures. An empty snapshot means "system
/// proxy unconfigured or unreadable" — callers treat both the same way.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct SystemProxySnapshot {
    pub http: Option<ProxyEndpoint>,
    pub https: Option<ProxyEndpoint>,
    /// The macOS proxy exception hosts (ExceptionsList), as entered.
    pub exceptions: Vec<String>,
}

impl SystemProxySnapshot {
    pub fn is_empty(&self) -> bool {
        self.http.is_none() && self.https.is_none() && self.exceptions.is_empty()
    }
}

/// Source of the system proxy state. Production reads SCDynamicStore; tests
/// inject their own snapshot.
pub trait SystemProxySource: Send + Sync {
    fn snapshot(&self) -> SystemProxySnapshot;
}

/// The production reader. A read failure yields an empty snapshot — the plan
/// then treats the system source as unconfigured (LW-809 D3), never as an
/// error surfaced to business calls.
#[derive(Default)]
pub struct ScStoreSource;

impl SystemProxySource for ScStoreSource {
    fn snapshot(&self) -> SystemProxySnapshot {
        read_sc_store()
    }
}

/// The production source behind the kernel.
pub fn production_source() -> Arc<dyn SystemProxySource> {
    Arc::new(ScStoreSource)
}

#[cfg(target_os = "macos")]
fn read_sc_store() -> SystemProxySnapshot {
    use system_configuration::core_foundation::array::CFArray;
    use system_configuration::core_foundation::base::TCFType;
    use system_configuration::core_foundation::number::CFNumber;
    use system_configuration::core_foundation::string::{CFString, CFStringRef};
    use system_configuration::dynamic_store::SCDynamicStoreBuilder;
    use system_configuration::sys::schema_definitions::{
        kSCPropNetProxiesHTTPEnable, kSCPropNetProxiesHTTPPort, kSCPropNetProxiesHTTPProxy,
        kSCPropNetProxiesHTTPSEnable, kSCPropNetProxiesHTTPSPort, kSCPropNetProxiesHTTPSProxy,
    };

    let empty = SystemProxySnapshot::default();
    let Some(store) = SCDynamicStoreBuilder::new("lulu-spark").build() else {
        return empty;
    };
    let Some(proxies) = store.get_proxies() else {
        return empty;
    };

    // Mirrors hyper-util's parsing: enabled == 1, a host is required, the
    // port may be absent.
    let scheme = |enable: CFStringRef, host: CFStringRef, port: CFStringRef| -> Option<ProxyEndpoint> {
        let enabled = proxies
            .find(enable)
            .and_then(|flag| flag.downcast::<CFNumber>())
            .and_then(|flag| flag.to_i32())
            .unwrap_or(0)
            == 1;
        if !enabled {
            return None;
        }
        let host = proxies
            .find(host)
            .and_then(|value| value.downcast::<CFString>())
            .map(|value| value.to_string())?;
        let port = proxies
            .find(port)
            .and_then(|value| value.downcast::<CFNumber>())
            .and_then(|value| value.to_i32())
            .map(|value| value as u16);
        Some(ProxyEndpoint { host, port })
    };

    // The crate does not export the exceptions key, and `CFArray<CFString>`
    // is not a ConcreteCFType — go through the void-pointer array and
    // re-wrap each entry. SCDynamicStore keys are content-equal CFStrings.
    let exceptions = proxies
        .find(&CFString::new("ExceptionsList"))
        .and_then(|value| value.downcast::<CFArray<*const std::ffi::c_void>>())
        .map(|list| {
            list.iter()
                .map(|item| unsafe { CFString::wrap_under_get_rule(*item as CFStringRef).to_string() })
                .collect()
        })
        .unwrap_or_default();

    SystemProxySnapshot {
        http: scheme(
            // Safety: extern CFString constants; contents are static.
            unsafe { kSCPropNetProxiesHTTPEnable },
            unsafe { kSCPropNetProxiesHTTPProxy },
            unsafe { kSCPropNetProxiesHTTPPort },
        ),
        https: scheme(
            unsafe { kSCPropNetProxiesHTTPSEnable },
            unsafe { kSCPropNetProxiesHTTPSProxy },
            unsafe { kSCPropNetProxiesHTTPSPort },
        ),
        exceptions,
    }
}

#[cfg(not(target_os = "macos"))]
fn read_sc_store() -> SystemProxySnapshot {
    SystemProxySnapshot::default()
}
