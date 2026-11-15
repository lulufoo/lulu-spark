//! The reqwest 0.13 factory over the same plan — the MCP SDK side of the
//! kernel boundary. The snapshot is taken at call time, which for MCP is the
//! transport-creation moment: a live session keeps its client, a reconnect
//! or new session picks up the current plan (LW-809 D6).

use reqwest013::redirect::Policy;
use reqwest013::{Client, NoProxy, Proxy};

use super::plan::ProxyPlan;
use super::{kernel, ClientSpec, NetError, RedirectSpec};

/// A client built from the current plan. MCP calls this when it creates a
/// transport; it must not cache the client across reconnects.
pub fn external(spec: &ClientSpec) -> Result<Client, NetError> {
    kernel().async_external(spec)
}

pub(super) fn build_client(plan: &ProxyPlan, spec: &ClientSpec) -> Result<Client, NetError> {
    // Same rule as the blocking side: the plan is authoritative, the
    // builder must not re-read env vars behind its back.
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
    builder = match spec.redirect {
        RedirectSpec::None => builder.redirect(Policy::none()),
        RedirectSpec::Default => builder,
    };
    if plan.http.is_some() || plan.https.is_some() {
        let no_proxy = || {
            NoProxy::from_string(&plan.no_proxy)
                .expect("no_proxy list is never empty: loopback entries are implicit")
        };
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
    }
    builder.build().map_err(|e| NetError::ClientBuild(e.to_string()))
}

