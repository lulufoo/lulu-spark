//! Per-business client requirements. The kernel keys its client cache on
//! this: businesses keep their own timeouts, redirects, and pool behavior;
//! what they share is the proxy strategy and its lifecycle.

use std::time::Duration;

/// Redirect handling a business requires. reqwest's `Policy` is not hashable,
/// so the spec stores the intent and the builder maps it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RedirectSpec {
    /// `Policy::none()` — the Board and cloud sync requirement.
    None,
    /// The reqwest default (up to 10 hops).
    Default,
}

/// One business client shape. Two businesses with equal specs share the same
/// kernel client (and its connections) within one generation.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ClientSpec {
    pub connect_timeout: Option<Duration>,
    pub request_timeout: Option<Duration>,
    pub redirect: RedirectSpec,
    pub pool_idle_timeout: Option<Duration>,
    /// Idle connections kept per host. `Some(0)` disables pooling — the MCP
    /// SDK default, kept to preserve its behavior.
    pub pool_max_idle_per_host: Option<u32>,
}

impl Default for ClientSpec {
    fn default() -> Self {
        Self {
            connect_timeout: None,
            request_timeout: None,
            redirect: RedirectSpec::None,
            pool_idle_timeout: None,
            pool_max_idle_per_host: None,
        }
    }
}

impl ClientSpec {
    pub fn with_connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = Some(timeout);
        self
    }

    pub fn with_request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = Some(timeout);
        self
    }

    pub fn with_redirect(mut self, redirect: RedirectSpec) -> Self {
        self.redirect = redirect;
        self
    }

    pub fn with_pool_idle_timeout(mut self, timeout: Duration) -> Self {
        self.pool_idle_timeout = Some(timeout);
        self
    }

    pub fn with_pool_max_idle_per_host(mut self, max: u32) -> Self {
        self.pool_max_idle_per_host = Some(max);
        self
    }
}
