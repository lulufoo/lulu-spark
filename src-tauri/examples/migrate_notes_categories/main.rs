//! One-off tool: import `notes/categories.json` into `note_category`, insert
//! `folder = ai` when missing, prove the row count, then delete the json file.
//!
//! Run by hand, once. The app never calls it and never reads the json file.
//!
//!   cargo run --example migrate_notes_categories -- --data-dir <dir>
//!
//! `<dir>` is the app data directory: it holds `notes/categories.json` and
//! `spark-data.sqlite`. Try it on a copy first.

mod migration;
#[cfg(test)]
mod tests;

use std::path::PathBuf;
use std::process::ExitCode;

fn read_data_dir() -> Result<PathBuf, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [flag, dir] if flag == "--data-dir" => Ok(PathBuf::from(dir)),
        _ => Err("usage: migrate_notes_categories --data-dir <dir>".to_string()),
    }
}

fn main() -> ExitCode {
    match read_data_dir().and_then(|dir| migration::run_migration(&dir)) {
        Ok(outcome) => {
            println!(
                "Imported {} categor(ies); inserted ai: {}; deleted categories.json.",
                outcome.row_count, outcome.inserted_ai
            );
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("Migration stopped: {message}");
            ExitCode::FAILURE
        }
    }
}
