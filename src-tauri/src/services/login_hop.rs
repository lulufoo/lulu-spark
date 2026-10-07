//! Login-chain hops on the process execution log (`app_log`).

use serde_json::{json, Map, Value};

use crate::services::app_log::{self, parse_trace_id, Level, Side};

pub const LOGIN_BUSINESS: &str = "login";
pub const LOGIN_EVENT_SCHEME_OPEN: &str = "scheme.open";
pub const PASS_QUERY: &str = "pass";
const SCHEME_MAX: usize = 512;

pub fn is_auth_login_scheme(scheme: &str) -> bool {
    scheme
        .strip_prefix("spark://")
        .is_some_and(|rest| rest.starts_with("auth-login/"))
}

pub fn pass_id_from_scheme(scheme: &str) -> Option<String> {
    let query = scheme.split_once('#').map(|(head, _)| head).unwrap_or(scheme);
    let query = query.split_once('?')?.1;
    for part in query.split('&') {
        let Some(raw) = part.strip_prefix("pass=") else {
            continue;
        };
        let decoded = urlencoding::decode(raw).ok()?;
        let bag: Value = serde_json::from_str(&decoded).ok()?;
        let id = bag.get("id")?.as_str()?;
        return parse_trace_id(id).map(str::to_string);
    }
    None
}

fn scheme_for_log(scheme: &str) -> String {
    let head = scheme.split_once('#').map(|(h, _)| h).unwrap_or(scheme);
    if head.len() > SCHEME_MAX {
        head[..SCHEME_MAX].to_string()
    } else {
        head.to_string()
    }
}

pub fn try_log_scheme_open(scheme: &str) -> bool {
    if !is_auth_login_scheme(scheme) {
        return false;
    }
    let id = pass_id_from_scheme(scheme).unwrap_or_else(|| "trace_missing".into());
    let mut params = Map::new();
    params.insert("scheme".into(), json!(scheme_for_log(scheme)));
    params.insert(
        "outcome".into(),
        json!(if scheme.is_empty() { "empty" } else { "ok" }),
    );
    app_log::log(
        LOGIN_BUSINESS,
        LOGIN_EVENT_SCHEME_OPEN,
        Some(&id),
        Some(Value::Object(params)),
        Side::Native,
        Level::Info,
    );
    true
}

#[cfg(test)]
#[path = "../unit-tests/services/login_hop.rs"]
mod tests;
