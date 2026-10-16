//! One-off tool: set `note.category_id` from the first segment of `raw_path`.
//!
//! Run by hand, once. The app never calls it.
//!
//!   cargo run --example migrate_note_category_id -- --data-dir <dir>
//!
//! `<dir>` holds `spark-data.sqlite`. Try it on a copy first.

mod migration;
#[cfg(test)]
mod tests;

use std::path::PathBuf;
use std::process::ExitCode;

fn read_data_dir() -> Result<PathBuf, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [flag, dir] if flag == "--data-dir" => Ok(PathBuf::from(dir)),
        _ => Err("usage: migrate_note_category_id --data-dir <dir>".to_string()),
    }
}

fn main() -> ExitCode {
    match read_data_dir().and_then(|dir| migration::run_migration(&dir)) {
        Ok(outcome) => {
            println!("Set category_id on {} note(s).", outcome.note_count);
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("Migration stopped: {message}");
            ExitCode::FAILURE
        }
    }
}
