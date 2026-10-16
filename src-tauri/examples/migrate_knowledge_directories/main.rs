//! One-off tool: import `knowledge/repos.json` into `knowledge_directory`,
//! skip folders without a disk directory plus `ai-dev-docs` and
//! `lulu-workflow-framework`, prove the row count, then delete both json files.
//!
//! Run by hand, once. The app never calls it and never reads the json files.
//!
//!   cargo run --example migrate_knowledge_directories -- --data-dir <dir>
//!
//! `<dir>` is the app data directory: it holds `knowledge/` and `spark-data.sqlite`.
//! Try it on a copy first.

mod migration;
#[cfg(test)]
mod tests;

use std::path::PathBuf;
use std::process::ExitCode;

fn read_data_dir() -> Result<PathBuf, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [flag, dir] if flag == "--data-dir" => Ok(PathBuf::from(dir)),
        _ => Err("usage: migrate_knowledge_directories --data-dir <dir>".to_string()),
    }
}

fn main() -> ExitCode {
    match read_data_dir().and_then(|dir| migration::run_migration(&dir)) {
        Ok(outcome) => {
            println!(
                "Imported {} director(ies); skipped {}; deleted repos.json and categories.json.",
                outcome.row_count, outcome.skipped
            );
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("Migration stopped: {message}");
            ExitCode::FAILURE
        }
    }
}
