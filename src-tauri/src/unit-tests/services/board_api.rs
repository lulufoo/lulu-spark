use std::sync::Arc;

use super::*;

#[test]
fn one_client_for_the_process_per_generation() {
    let a = client().expect("client");
    let b = client().expect("client");
    assert!(
        Arc::ptr_eq(&a, &b),
        "the kernel must hand back the same client within a generation"
    );
}

#[test]
fn warmup_is_spawned_from_host_boot_and_does_not_send_a_pat() {
    let lib = include_str!("../../lib.rs");
    let api = include_str!("../../services/board_api.rs");
    let sketch = include_str!("../../services/board_sketch.rs");
    assert!(lib.contains("board_api::spawn_warm"));
    assert!(api.contains("board_sketch_log::warmup"));
    assert!(!api.contains("AUTHORIZATION"));
    assert!(!api.contains("Bearer"));
    assert!(!api.contains("lbp_"));
    assert!(!api.contains("app_log::log"));
    assert!(sketch.contains("board_api::client"));
    assert!(!sketch.contains("Client::builder"));
}
