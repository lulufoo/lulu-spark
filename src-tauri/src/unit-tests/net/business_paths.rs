//! Business-path evidence for LW-812: a Board sketch read issued by the real
//! call chain leaves its CONNECT record at the observable proxy.

use super::kernel::ObservableProxy;
use super::{lock_env, set_env, EnvCleaner};

#[test]
fn board_sketch_read_travels_through_the_kernel_client() {
    let _lock = lock_env();
    let proxy = ObservableProxy::start();
    {
        let _clean = EnvCleaner;
        set_env("HTTPS_PROXY", &proxy.env_value());
        crate::net::refresh_now();
        let fetch = crate::services::board_sketch::fetch_document("lbp_pat", "b_12345678");
        assert!(fetch.bytes.is_none(), "the proxy stand-in does not speak Board JSON");
        let hits = proxy.hits();
        assert_eq!(hits.len(), 1, "exactly one proxied request: {hits:?}");
        assert!(
            hits[0].starts_with("CONNECT api.luluboard.app:443"),
            "Board evidence is the CONNECT record: {}",
            hits[0]
        );
    }
    crate::net::refresh_now();
}

/// LW-813: the inventory stays closed — no business module may keep a
/// client builder that bypasses the kernel.
#[test]
fn production_client_creation_stays_inside_the_kernel() {
    for (path, source) in [
        (
            "agent/llm.rs",
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/agent/llm.rs")),
        ),
        (
            "gateway/mod.rs",
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/gateway/mod.rs")),
        ),
        (
            "mcp_host/server/listen.rs",
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/mcp_host/server/listen.rs")),
        ),
        (
            "services/link_title.rs",
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/services/link_title.rs")),
        ),
        (
            "services/board_api.rs",
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/services/board_api.rs")),
        ),
        (
            "services/cloud_sync/client.rs",
            include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/src/services/cloud_sync/client.rs")),
        ),
    ] {
        assert!(
            !source.contains("Client::builder"),
            "{path} must take its client from the net kernel"
        );
        assert!(
            !source.contains("Client::new("),
            "{path} must take its client from the net kernel"
        );
    }
}
