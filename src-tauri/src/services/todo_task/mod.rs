//! Todo tasks persisted at `{workbench_knowledge_root}/todo_tasks/` (v2: index + per-task files).

pub mod types;

mod attachments;
mod categories;
mod comments;
mod error;
mod master;
mod migrate;
mod plan_md;
mod store;
mod subs;

#[cfg(test)]
mod test_hooks;
#[cfg(test)]
mod test_wire;

pub use error::{into_wire, into_wire_read, TodoError};

#[cfg(test)]
pub use store::test_run_write_task_batch;
#[cfg(test)]
pub use test_hooks::{
    test_reset_all_injection_flags, test_set_fail_add_attachment_manifest,
    test_set_fail_batch_index, test_set_fail_batch_sub_tasks, test_set_fail_batch_todo_md,
    test_set_fail_complete_sub, test_set_fail_delete_attachment_file, test_set_fail_link_archive,
    test_set_fail_migrate_implicit_write,
};

pub use attachments::{
    add_attachment, delete_attachment, list_attachments, read_attachment, save_attachment,
    stage_attachment_source,
};
pub use categories::{
    create_todo_category, delete_todo_category, ensure_default_category, list_todo_categories,
    load_categories, set_master_category,
};
pub use comments::{add_comment, delete_comment, list_comments, update_comment};
pub use master::{
    create_master_with_category, create_master_with_subs, create_master_with_subs_and_todo,
    delete_master, get_by_id, list_all, set_master_status, title_unit_count, update_master_fields,
    update_master_title,
};
pub use migrate::{
    ensure_todo_api_ungated, migrate_todos_default_category, migration_gate_passed,
    MIGRATION_GATE_FILE,
};
pub use plan_md::{read_todo_md, update_todo_md};
pub use subs::{
    abandon_sub, add_sub, complete_sub, complete_todo, delete_sub, link_archive, update_sub_title,
};

#[cfg(test)]
#[path = "../../unit-tests/services/todo_task/mod.rs"]
mod tests;
