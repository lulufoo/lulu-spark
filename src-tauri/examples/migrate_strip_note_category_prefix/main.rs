//! One-off tool: strip the category folder from stored `raw_path` / `digest_path`.
//!
//! Disk files stay put. The app never calls this.
//!
//!   cargo run --example migrate_strip_note_category_prefix -- --data-dir <dir>
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
        _ => Err("usage: migrate_strip_note_category_prefix --data-dir <dir>".to_string()),
    }
}

fn main() -> ExitCode {
    match read_data_dir().and_then(|dir| migration::run_migration(&dir)) {
        Ok(outcome) => {
            println!(
                "Stripped category prefix from {} note path(s).",
                outcome.path_count
            );
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("Migration stopped: {message}");
            ExitCode::FAILURE
        }
    }
}
