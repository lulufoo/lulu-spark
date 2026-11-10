//! Desktop read of one Board Sketch. The reply carries a status and, for a
//! Sketch, its content Base64. It never carries the PAT.

use tauri::AppHandle;

use crate::services::board_sketch::{read_board_sketch as load_sketch, BoardSketchReply};

#[tauri::command]
pub fn read_board_sketch(_app: AppHandle, url: String) -> BoardSketchReply {
    load_sketch(&url)
}
