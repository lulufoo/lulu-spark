//! Desktop read of one Board Sketch. The reply carries a status and, for a
//! Sketch, its content Base64. It never carries the PAT.

use tauri::AppHandle;

use crate::services::board_cache;
use crate::services::board_sketch::{failure, BoardSketchReply};

pub async fn load_board_sketch(url: String) -> BoardSketchReply {
    tauri::async_runtime::spawn_blocking(move || board_cache::read_board_sketch(&url))
        .await
        .unwrap_or_else(|_| failure("network"))
}

pub async fn load_cached_board_sketch(url: String) -> BoardSketchReply {
    tauri::async_runtime::spawn_blocking(move || board_cache::read_cached_board_sketch(&url))
        .await
        .unwrap_or_else(|_| failure("cache_miss"))
}

#[tauri::command]
pub async fn read_board_sketch(_app: AppHandle, url: String) -> BoardSketchReply {
    load_board_sketch(url).await
}

#[tauri::command]
pub async fn read_cached_board_sketch(_app: AppHandle, url: String) -> BoardSketchReply {
    load_cached_board_sketch(url).await
}

#[cfg(test)]
#[path = "../unit-tests/commands/board_sketch.rs"]
mod tests;
