//! Desktop commands for the Board personal access token.
//!
//! Callers learn only whether a token is stored. These commands never return it.

use tauri::AppHandle;

use crate::config::vault::{self, SecretError};

pub fn read_board_pat_status() -> Result<bool, String> {
    vault::board_pat_configured().map_err(secret_message)
}

pub fn write_board_pat(pat: &str) -> Result<(), String> {
    crate::services::board_cache::set_board_pat(pat)
}

pub fn remove_board_pat() -> Result<(), String> {
    crate::services::board_cache::delete_board_pat()
}

fn secret_message(err: SecretError) -> String {
    err.to_string()
}

#[tauri::command]
pub fn get_board_pat_status(_app: AppHandle) -> Result<bool, String> {
    read_board_pat_status()
}

#[tauri::command]
pub fn set_board_pat(_app: AppHandle, pat: String) -> Result<(), String> {
    write_board_pat(&pat)
}

#[tauri::command]
pub fn delete_board_pat(_app: AppHandle) -> Result<(), String> {
    remove_board_pat()
}

#[cfg(test)]
#[path = "../unit-tests/commands/board_pat.rs"]
mod tests;
