use std::thread;

use super::{load_board_sketch, load_cached_board_sketch};

#[test]
fn the_read_runs_off_the_caller_thread() {
    let caller = thread::current().id();
    let (reply, worker) = tauri::async_runtime::block_on(async {
        tauri::async_runtime::spawn_blocking(|| {
            let worker = thread::current().id();
            let reply = crate::services::board_cache::read_board_sketch(
                "https://evil.test/#b:b_0123abcd",
            );
            (reply, worker)
        })
        .await
        .expect("read task")
    });
    assert_eq!(reply.status, "not_board");
    assert!(reply.content_b64.is_none());
    assert_eq!(reply.version, None);
    assert_eq!(reply.invalidation_scope, None);
    assert_ne!(caller, worker);
    let wrapped = tauri::async_runtime::block_on(load_board_sketch(
        "https://evil.test/#b:b_0123abcd".into(),
    ));
    assert_eq!(wrapped.status, "not_board");
    assert_eq!(wrapped.version, None);
    assert_eq!(wrapped.invalidation_scope, None);
}

#[test]
fn cache_read_uses_the_same_url_boundary_without_a_network_fallback() {
    for url in [
        "https://evil.test/#b:b_0123abcd",
        "https://luluboard.app/#b:invalid",
    ] {
        let reply = tauri::async_runtime::block_on(load_cached_board_sketch(url.into()));
        assert!(matches!(reply.status, "not_board" | "invalid_id"));
        assert!(reply.content_b64.is_none());
        assert!(reply.version.is_none());
        assert!(reply.invalidation_scope.is_none());
    }
}
