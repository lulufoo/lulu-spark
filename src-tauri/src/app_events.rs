//! Wire service callbacks to desktop events; services do not depend on Tauri.

use std::sync::Arc;
use tauri::Emitter;

pub fn install(app: &tauri::AppHandle) {
    let messages = app.clone();
    crate::services::message_center::set_changed_handler(Some(Arc::new(move || {
        if let Some(envelope) = crate::services::message_center::last_changed_envelope() {
            let _ = messages.emit("message-center:changed", envelope);
        }
    })));
    let board = app.clone();
    crate::services::board_cache::set_invalidated_handler(Some(Arc::new(move |event| {
        let _ = board.emit("board-cache:invalidated", event);
    })));
}
