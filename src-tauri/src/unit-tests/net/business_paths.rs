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
