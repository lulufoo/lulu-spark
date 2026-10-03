use super::*;

#[test]
fn job_state_idle_has_expected_fields() {
    let j = JobState::idle();
    assert_eq!(j.status, "idle");
    assert!(j.started_at.is_none());
    assert!(j.finished_at.is_none());
    assert!(j.log.is_empty());
    let v = j.to_json();
    assert_eq!(v["status"], "idle");
    assert!(v.get("log").is_some());
}

#[test]
fn no_python3_spawn_in_reindex_module() {
    let src = include_str!("reindex.rs");
    assert!(!src.contains("Command::new(\"python3\")"));
    assert!(src.contains("rebuild_spark_index"));
    assert!(src.contains("rebuild_knowledge_index"));
}
