//! ProxyPlan: the pure resolution of (toggle, environment, system snapshot)
//! into per-scheme proxy decisions and the merged no-proxy set.
//!
//! Precedence (LW-809 D2), mirroring hyper-util's own `from_system()`
//! semantics: `NO_PROXY` rules always apply; for one scheme the specific env
//! variable wins over `ALL_PROXY`, which wins over the system proxy — and
//! the system proxy counts only while the Network toggle is on. Loopback
//! targets are never proxied: the implicit exclusions at the front of
//! `no_proxy` guarantee that regardless of what the user configured.

use serde_json::{json, Value};

use crate::host::{ProxyEndpoint, SystemProxySnapshot};

/// Loopback is always direct, per the locked product requirement.
pub const LOOPBACK_HOSTS: [&str; 3] = ["localhost", "127.0.0.1", "::1"];

/// Where one scheme's proxy decision came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProxySource {
    Env,
    System,
}

/// One resolved proxy decision: the URL reqwest receives (credentials kept),
/// plus a credentials-free `display` for logs and snapshots.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedProxy {
    pub url: String,
    pub display: String,
    pub source: ProxySource,
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProxyPlan {
    pub http: Option<ResolvedProxy>,
    pub https: Option<ResolvedProxy>,
    /// Merged exclusion list for `NoProxy::from_string`: implicit loopback,
    /// then `NO_PROXY`, then (toggle on) the system ExceptionsList.
    pub no_proxy: String,
}

impl ProxyPlan {
    pub fn direct() -> Self {
        Self {
            no_proxy: LOOPBACK_HOSTS.join(","),
            ..Self::default()
        }
    }
}

/// Inputs of one resolution. `system` is the empty snapshot whenever the
/// toggle is off or the read failed — callers never distinguish the two.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PlanInputs {
    pub use_system_proxy: bool,
    pub system: SystemProxySnapshot,
}

/// The plan plus the dropped-source warnings it produced.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resolved {
    pub plan: ProxyPlan,
    pub warnings: Vec<String>,
}

/// Env vars as hyper-util reads them: uppercase first, then lowercase.
struct EnvProxies {
    all: Option<String>,
    http: Option<String>,
    https: Option<String>,
    no_proxy: Option<String>,
}

fn first_env(names: &[&str]) -> Option<String> {
    names
        .iter()
        .find_map(|name| std::env::var(name).ok())
        .filter(|value| !value.trim().is_empty())
}

/// `None` under a CGI environment (`REQUEST_METHOD` set): env proxies are
/// ignored there, mirroring hyper-util.
fn snapshot_env() -> Option<EnvProxies> {
    if std::env::var_os("REQUEST_METHOD").is_some() {
        return None;
    }
    Some(EnvProxies {
        all: first_env(&["ALL_PROXY", "all_proxy"]),
        http: first_env(&["HTTP_PROXY", "http_proxy"]),
        https: first_env(&["HTTPS_PROXY", "https_proxy"]),
        no_proxy: first_env(&["NO_PROXY", "no_proxy"]),
    })
}

/// Resolve one raw value into a usable proxy URL. Schemes other than
/// http/https (socks included) are out of scope this round and count as an
/// invalid source, per the contract.
fn parse_proxy(raw: &str) -> Option<ResolvedProxy> {
    let raw = raw.trim();
    if raw.is_empty() || raw.split("://").next().unwrap_or("").starts_with("socks") {
        return None;
    }
    let candidate = if raw.contains("://") {
        raw.to_string()
    } else {
        format!("http://{raw}")
    };
    let url = reqwest::Url::parse(&candidate).ok()?;
    if !matches!(url.scheme(), "http" | "https") {
        return None;
    }
    let host = url.host_str().filter(|host| !host.is_empty())?;
    let display = match url.port() {
        Some(port) => format!("{host}:{port}"),
        None => host.to_string(),
    };
    Some(ResolvedProxy {
        url: candidate,
        display,
        source: ProxySource::Env,
    })
}

fn parse_env_source(raw: &Option<String>, source: &str, warnings: &mut Vec<String>) -> Option<ResolvedProxy> {
    let raw = raw.as_deref()?;
    match parse_proxy(raw) {
        Some(mut proxy) => {
            proxy.source = ProxySource::Env;
            Some(proxy)
        }
        None => {
            warnings.push(format!("dropped invalid proxy from {source}"));
            None
        }
    }
}

fn parse_system_endpoint(
    endpoint: Option<&ProxyEndpoint>,
    scheme: &str,
    warnings: &mut Vec<String>,
) -> Option<ResolvedProxy> {
    let endpoint = endpoint?;
    let raw = match endpoint.port {
        Some(port) => format!("http://{}:{port}", endpoint.host),
        None => format!("http://{}", endpoint.host),
    };
    match parse_proxy(&raw) {
        Some(mut proxy) => {
            proxy.source = ProxySource::System;
            Some(proxy)
        }
        None => {
            warnings.push(format!("dropped invalid system {scheme} proxy"));
            None
        }
    }
}

fn merge_no_proxy(env_no: Option<&str>, exceptions: &[String]) -> String {
    let mut parts: Vec<String> = LOOPBACK_HOSTS.iter().map(|host| host.to_string()).collect();
    if let Some(list) = env_no {
        parts.extend(list.split(',').map(str::trim).filter(|part| !part.is_empty()).map(String::from));
    }
    parts.extend(exceptions.iter().map(|part| part.trim().to_string()).filter(|part| !part.is_empty()));
    let mut seen = std::collections::HashSet::new();
    let mut merged = Vec::with_capacity(parts.len());
    for part in parts {
        if seen.insert(part.clone()) {
            merged.push(part);
        }
    }
    merged.join(",")
}

/// Resolve one plan. Pure: the environment is snapshotted here, the system
/// snapshot comes in through `inputs`.
pub fn resolve(inputs: &PlanInputs) -> Resolved {
    let mut warnings = Vec::new();
    let env = snapshot_env();

    let env_http = env.as_ref().and_then(|env| parse_env_source(&env.http, "HTTP_PROXY", &mut warnings));
    let env_all = env.as_ref().and_then(|env| parse_env_source(&env.all, "ALL_PROXY", &mut warnings));
    let env_https = env.as_ref().and_then(|env| parse_env_source(&env.https, "HTTPS_PROXY", &mut warnings));

    let system = if inputs.use_system_proxy { Some(&inputs.system) } else { None };
    let system_http = system.and_then(|snapshot| parse_system_endpoint(snapshot.http.as_ref(), "http", &mut warnings));
    let system_https = system.and_then(|snapshot| parse_system_endpoint(snapshot.https.as_ref(), "https", &mut warnings));

    let plan = ProxyPlan {
        // Specific env, then all, then system — an invalid source drops out
        // and the next one takes over (LW-809 D3).
        http: env_http.or_else(|| env_all.clone()).or(system_http),
        https: env_https.or(env_all).or(system_https),
        no_proxy: merge_no_proxy(env.as_ref().and_then(|env| env.no_proxy.as_deref()), &inputs.system.exceptions),
    };
    Resolved { plan, warnings }
}

/// Credentials-free summary for logs and `get_config` snapshots.
pub fn summary_json(plan: &ProxyPlan) -> Value {
    let endpoint = |proxy: &Option<ResolvedProxy>| match proxy {
        None => Value::Null,
        Some(proxy) => json!({
            "host_port": proxy.display,
            "source": match proxy.source {
                ProxySource::Env => "env",
                ProxySource::System => "system",
            },
        }),
    };
    json!({
        "http": endpoint(&plan.http),
        "https": endpoint(&plan.https),
        "no_proxy": plan.no_proxy,
    })
}

