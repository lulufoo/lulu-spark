//! Test-only Value facade so existing service tests keep asserting the L1 wire.

#![allow(dead_code, unused_imports)]

use serde_json::Value;

use super::{into_wire, into_wire_read, TodoError};

pub(crate) use super::comments::{
    load_comments_file_unlocked, mint_comment_id, now_comment_created_at,
    save_comments_file_unlocked, validate_comment_body,
};
pub(crate) use super::{
    ensure_default_category, load_categories, test_reset_all_injection_flags,
    test_run_write_task_batch, test_set_fail_add_attachment_manifest, test_set_fail_batch_index,
    test_set_fail_batch_sub_tasks, test_set_fail_batch_todo_md, test_set_fail_complete_sub,
    test_set_fail_delete_attachment_file, test_set_fail_link_archive,
    test_set_fail_migrate_implicit_write, title_unit_count,
};

fn w(result: Result<Value, TodoError>, ok_status: u16) -> Value {
    into_wire(result, ok_status)
}

fn wr(result: Result<Value, TodoError>) -> Value {
    into_wire_read(result)
}

pub fn create_master_with_subs(title: &str, sub_titles: Option<&[&str]>) -> Value {
    w(super::create_master_with_subs(title, sub_titles), 201)
}

pub fn create_master_with_subs_and_todo(
    title: &str,
    sub_titles: Option<&[&str]>,
    plan_md: &str,
) -> Value {
    w(
        super::create_master_with_subs_and_todo(title, sub_titles, plan_md),
        201,
    )
}

pub fn create_master_with_category(
    title: &str,
    sub_titles: Option<&[&str]>,
    plan_md: &str,
    category_id: Option<&str>,
) -> Value {
    w(
        super::create_master_with_category(title, sub_titles, plan_md, category_id),
        201,
    )
}

pub fn get_by_id(id: &str) -> Value {
    wr(super::get_by_id(id))
}

pub fn list_all() -> Value {
    wr(super::list_all())
}

pub fn delete_master(master_task_id: &str) -> Value {
    w(super::delete_master(master_task_id), 200)
}

pub fn update_master_title(master_task_id: &str, title: &str) -> Value {
    w(super::update_master_title(master_task_id, title), 200)
}

pub fn update_master_fields(
    master_task_id: &str,
    title: Option<&str>,
    todo_md: Option<&str>,
) -> Value {
    w(super::update_master_fields(master_task_id, title, todo_md), 200)
}

pub fn set_master_status(master_task_id: &str, status: &str) -> Value {
    w(super::set_master_status(master_task_id, status), 200)
}

pub fn add_sub(master_task_id: &str, title: &str, content: Option<&str>) -> Value {
    w(super::add_sub(master_task_id, title, content), 201)
}

pub fn delete_sub(master_task_id: &str, sub_task_id: &str) -> Value {
    w(super::delete_sub(master_task_id, sub_task_id), 200)
}

pub fn complete_sub(master_task_id: &str, sub_task_id: &str) -> Value {
    w(super::complete_sub(master_task_id, sub_task_id), 200)
}

pub fn complete_todo(master_task_id: &str, sub_task_id: Option<&str>) -> Value {
    w(super::complete_todo(master_task_id, sub_task_id), 200)
}

pub fn update_sub_title(
    master_task_id: &str,
    sub_task_id: &str,
    title: &str,
    content: Option<&str>,
) -> Value {
    w(
        super::update_sub_title(master_task_id, sub_task_id, title, content),
        200,
    )
}

pub fn abandon_sub(master_task_id: &str, sub_task_id: &str) -> Value {
    w(super::abandon_sub(master_task_id, sub_task_id), 200)
}

pub fn link_archive(master_task_id: &str, sub_task_id: &str, archive_id: &str) -> Value {
    w(super::link_archive(master_task_id, sub_task_id, archive_id), 200)
}

pub fn read_todo_md(master_task_id: &str) -> Value {
    w(super::read_todo_md(master_task_id), 200)
}

pub fn update_todo_md(master_task_id: &str, plan_md: &str) -> Value {
    w(super::update_todo_md(master_task_id, plan_md), 200)
}

pub fn list_todo_categories() -> Value {
    w(super::list_todo_categories(), 200)
}

pub fn create_todo_category(name: &str) -> Value {
    w(super::create_todo_category(name), 201)
}

pub fn delete_todo_category(category_id: &str) -> Value {
    w(super::delete_todo_category(category_id), 200)
}

pub fn set_master_category(master_task_id: &str, category_id: &str) -> Value {
    w(super::set_master_category(master_task_id, category_id), 200)
}

pub fn migrate_todos_default_category() -> Value {
    w(super::migrate_todos_default_category(), 200)
}

pub fn stage_attachment_source(preferred_name: &str, content: &str) -> Value {
    w(super::stage_attachment_source(preferred_name, content), 201)
}

pub fn add_attachment(master_task_id: &str, source_path: &str) -> Value {
    w(super::add_attachment(master_task_id, source_path), 201)
}

pub fn list_attachments(master_task_id: &str) -> Value {
    w(super::list_attachments(master_task_id), 200)
}

pub fn read_attachment(master_task_id: &str, file_name: &str) -> Value {
    w(super::read_attachment(master_task_id, file_name), 200)
}

pub fn save_attachment(master_task_id: &str, file_name: &str, source_path: &str) -> Value {
    w(
        super::save_attachment(master_task_id, file_name, source_path),
        200,
    )
}

pub fn delete_attachment(master_task_id: &str, file_name: &str) -> Value {
    w(super::delete_attachment(master_task_id, file_name), 200)
}

pub fn list_comments(master_task_id: &str) -> Value {
    w(super::list_comments(master_task_id), 200)
}

pub fn add_comment(master_task_id: &str, body: &str) -> Value {
    w(super::add_comment(master_task_id, body), 201)
}

pub fn update_comment(master_task_id: &str, comment_id: &str, body: &str) -> Value {
    w(super::update_comment(master_task_id, comment_id, body), 200)
}

pub fn delete_comment(master_task_id: &str, comment_id: &str) -> Value {
    w(super::delete_comment(master_task_id, comment_id), 200)
}
