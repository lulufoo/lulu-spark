use super::*;
use crate::services::app_log::token_ok;
use std::time::Duration;

#[test]
fn business_id_is_a_token_app_log_accepts() {
    assert_eq!(BUSINESS, "board_sketch");
    assert!(token_ok(BUSINESS));
    assert!(token_ok(EVENT_READ));
}

#[test]
fn as_ms_caps_subsecond_durations() {
    assert_eq!(as_ms(Duration::from_millis(0)), 0);
    assert_eq!(as_ms(Duration::from_millis(12)), 12);
}

#[test]
fn production_files_log_under_this_business_and_never_log_secrets() {
    let helper = include_str!("../../services/board_sketch_log.rs");
    assert!(helper.contains("#[path = \"../unit-tests/services/board_sketch_log.rs\"]"));
    assert!(!helper.contains("#[test]"));
    assert!(helper.contains("app_log::log(BUSINESS"));

    let prod = include_str!("../../services/board_sketch.rs");
    assert!(prod.contains("board_sketch_log::read"));
    assert!(!prod.contains("app_log::log"));
    for key in ["lbp_", "AUTHORIZATION", "Bearer", "content_b64"] {
        assert!(!helper.contains(key), "log helper must not mention {key}");
    }
}
