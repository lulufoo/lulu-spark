//! Log fields shared by the MCP Host OAuth routes (business id `mcp_oauth`).

use axum::http::{header, HeaderMap};

use crate::services::mcp_oauth_log::clip;

/// Who is calling, e.g. `Cursor/3.27.12 (darwin arm64)`. Empty when absent.
pub(super) fn agent(headers: &HeaderMap) -> String {
    clip(
        headers
            .get(header::USER_AGENT)
            .and_then(|value| value.to_str().ok())
            .unwrap_or(""),
    )
}
