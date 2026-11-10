//! Desktop read of one Board Sketch. The reply carries a status and, for a
//! Sketch, its content Base64. It never carries the PAT.

use tauri::AppHandle;

use crate::services::board_sketch::{read_board_sketch as load_sketch, BoardSketchReply};

pub async fn load_board_sketch(url: String) -> BoardSketchReply {
    match tauri::async_runtime::spawn_blocking(move || load_sketch(&url)).await {
        Ok(reply) => reply,
        Err(_) => BoardSketchReply {
            status: "network",
            content_b64: None,
            title: None,
        },
    }
}

#[tauri::command]
pub async fn read_board_sketch(_app: AppHandle, url: String) -> BoardSketchReply {
    load_board_sketch(url).await
}

#[cfg(test)]
#[path = "../unit-tests/commands/board_sketch.rs"]
mod tests;
