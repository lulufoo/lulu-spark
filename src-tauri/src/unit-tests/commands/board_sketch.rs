use std::thread;

use super::load_board_sketch;

#[test]
fn the_read_runs_off_the_caller_thread() {
    let caller = thread::current().id();
    let (reply, worker) = tauri::async_runtime::block_on(async {
        tauri::async_runtime::spawn_blocking(|| {
            let worker = thread::current().id();
            let reply = crate::services::board_sketch::read_board_sketch("https://evil.test/#b:b_0123abcd");
            (reply, worker)
        })
        .await
        .expect("read task")
    });
    assert_eq!(reply.status, "not_board");
    assert!(reply.content_b64.is_none());
    assert_ne!(caller, worker);
    let wrapped = tauri::async_runtime::block_on(load_board_sketch(
        "https://evil.test/#b:b_0123abcd".into(),
    ));
    assert_eq!(wrapped.status, "not_board");
}
