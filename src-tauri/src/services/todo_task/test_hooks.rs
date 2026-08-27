//! Test-only failure injection for todo_task persistence paths.

use std::sync::atomic::{AtomicBool, Ordering};

#[cfg(test)]
pub(super) static TEST_FAIL_COMPLETE_SUB: AtomicBool = AtomicBool::new(false);

#[cfg(test)]
pub(super) static TEST_FAIL_LINK_ARCHIVE: AtomicBool = AtomicBool::new(false);

#[cfg(test)]
pub(super) static TEST_FAIL_BATCH_SUB_TASKS: AtomicBool = AtomicBool::new(false);

#[cfg(test)]
pub(super) static TEST_FAIL_BATCH_PLAN_MD: AtomicBool = AtomicBool::new(false);

#[cfg(test)]
pub(super) static TEST_FAIL_BATCH_INDEX: AtomicBool = AtomicBool::new(false);

#[cfg(test)]
pub(super) static TEST_FAIL_MIGRATE_IMPLICIT_WRITE: AtomicBool = AtomicBool::new(false);

#[cfg(test)]
pub(super) static TEST_FAIL_ADD_ATTACHMENT_MANIFEST: AtomicBool = AtomicBool::new(false);

#[cfg(test)]
pub(super) static TEST_FAIL_DELETE_ATTACHMENT_FILE: AtomicBool = AtomicBool::new(false);

#[cfg(test)]
pub fn test_set_fail_complete_sub(fail: bool) {
    TEST_FAIL_COMPLETE_SUB.store(fail, Ordering::SeqCst);
}

#[cfg(test)]
pub fn test_set_fail_link_archive(fail: bool) {
    TEST_FAIL_LINK_ARCHIVE.store(fail, Ordering::SeqCst);
}

#[cfg(test)]
pub fn test_set_fail_batch_sub_tasks(fail: bool) {
    TEST_FAIL_BATCH_SUB_TASKS.store(fail, Ordering::SeqCst);
}

#[cfg(test)]
pub fn test_set_fail_batch_todo_md(fail: bool) {
    TEST_FAIL_BATCH_PLAN_MD.store(fail, Ordering::SeqCst);
}

#[cfg(test)]
pub fn test_set_fail_batch_index(fail: bool) {
    TEST_FAIL_BATCH_INDEX.store(fail, Ordering::SeqCst);
}

#[cfg(test)]
pub fn test_set_fail_migrate_implicit_write(fail: bool) {
    TEST_FAIL_MIGRATE_IMPLICIT_WRITE.store(fail, Ordering::SeqCst);
}

#[cfg(test)]
pub fn test_set_fail_add_attachment_manifest(fail: bool) {
    TEST_FAIL_ADD_ATTACHMENT_MANIFEST.store(fail, Ordering::SeqCst);
}

#[cfg(test)]
pub fn test_set_fail_delete_attachment_file(fail: bool) {
    TEST_FAIL_DELETE_ATTACHMENT_FILE.store(fail, Ordering::SeqCst);
}

#[cfg(test)]
pub fn test_reset_all_injection_flags() {
    TEST_FAIL_COMPLETE_SUB.store(false, Ordering::SeqCst);
    TEST_FAIL_LINK_ARCHIVE.store(false, Ordering::SeqCst);
    TEST_FAIL_BATCH_SUB_TASKS.store(false, Ordering::SeqCst);
    TEST_FAIL_BATCH_PLAN_MD.store(false, Ordering::SeqCst);
    TEST_FAIL_BATCH_INDEX.store(false, Ordering::SeqCst);
    TEST_FAIL_MIGRATE_IMPLICIT_WRITE.store(false, Ordering::SeqCst);
    TEST_FAIL_ADD_ATTACHMENT_MANIFEST.store(false, Ordering::SeqCst);
    TEST_FAIL_DELETE_ATTACHMENT_FILE.store(false, Ordering::SeqCst);
}

#[cfg(test)]
pub(super) fn test_take_fail_link_archive() -> bool {
    TEST_FAIL_LINK_ARCHIVE.swap(false, Ordering::SeqCst)
}

#[cfg(test)]
pub(super) fn test_take_fail_batch_sub_tasks() -> bool {
    TEST_FAIL_BATCH_SUB_TASKS.swap(false, Ordering::SeqCst)
}

#[cfg(test)]
pub(super) fn test_take_fail_batch_plan_md() -> bool {
    TEST_FAIL_BATCH_PLAN_MD.swap(false, Ordering::SeqCst)
}

#[cfg(test)]
pub(super) fn test_take_fail_batch_index() -> bool {
    TEST_FAIL_BATCH_INDEX.swap(false, Ordering::SeqCst)
}

#[cfg(test)]
pub(super) fn test_take_fail_migrate_implicit_write() -> bool {
    TEST_FAIL_MIGRATE_IMPLICIT_WRITE.swap(false, Ordering::SeqCst)
}

#[cfg(test)]
pub(super) fn test_take_fail_complete_sub() -> bool {
    TEST_FAIL_COMPLETE_SUB.swap(false, Ordering::SeqCst)
}

#[cfg(test)]
pub(super) fn test_take_fail_add_attachment_manifest() -> bool {
    TEST_FAIL_ADD_ATTACHMENT_MANIFEST.swap(false, Ordering::SeqCst)
}

#[cfg(test)]
pub(super) fn test_take_fail_delete_attachment_file() -> bool {
    TEST_FAIL_DELETE_ATTACHMENT_FILE.swap(false, Ordering::SeqCst)
}
