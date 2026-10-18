//! The Home chat list's launch cutoff, and what counts as activity on a chat.

use serde_json::Value;

use super::catalog;
use super::live::with_live_mut;
use super::schema::{checked_session_id, unix_secs};
use super::session_db;
use super::store::{catalog_file_path, session_file_path};

/// Disk session catalog for the Home chat list: the chats from the launch cutoff on, newest first.
/// Before the cutoff is fixed (`init_recent_cutoff`) there is no cutoff and every chat is listed.
pub fn list_session_summaries() -> Result<Vec<Value>, String> {
    let cutoff = with_live_mut(|live| live.recent_cutoff).unwrap_or(0);
    catalog::list_since(&catalog_file_path()?, cutoff)
}

/// Fix the Home list's cutoff from the chats on disk: local midnight of the older of the
/// `RECENT_CHAT_DAYS` most recent days that have chats. Called once at launch; it is not moved
/// while the app runs, so a chat started or opened later never pushes a day out of the list.
pub fn init_recent_cutoff() -> Result<i64, String> {
    let cutoff = catalog::recent_cutoff(&catalog_file_path()?, catalog::RECENT_CHAT_DAYS)?;
    with_live_mut(|live| live.recent_cutoff = Some(cutoff));
    Ok(cutoff)
}

/// Count opening a chat as activity: its `updated_at` becomes now. The title and the turns stay
/// as they are, so this must not go through `save_session`, which rebuilds the title.
pub fn touch_session(session_id: &str) -> Result<(), String> {
    let id = checked_session_id(session_id)?;
    let now = unix_secs();
    let path = session_file_path(id)?;
    if !path.is_file() {
        // Opening a missing file would create an empty session database.
        return Err(format!("chat session not found: {id}"));
    }
    session_db::touch(&path, now)?;
    catalog::touch(&catalog_file_path()?, id, now)
}
