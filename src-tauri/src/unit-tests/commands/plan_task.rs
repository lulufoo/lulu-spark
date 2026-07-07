use crate::commands::plan_task::get_plan_tasks_json;
use crate::services::plan_task::{create_master_with_subs, list_all};
use crate::test_support::with_config_test_serial;

#[test]
fn get_plan_tasks_json_returns_desc_sorted_array() {
    with_config_test_serial(|| {
        let created = create_master_with_subs("Plan A", None);
        assert!(created.get("master_task_id").is_some());
        let listed = get_plan_tasks_json().expect("list");
        let arr = listed.as_array().expect("array");
        assert!(!arr.is_empty());
        assert_eq!(list_all(), listed);
    });
}
