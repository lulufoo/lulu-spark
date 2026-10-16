//! One-off tool: set `knowledge_doc.directory_id` from `knowledge_directory.folder`.
//!
//! Run by hand, once. The app never calls it.
//!
//!   cargo run --example migrate_knowledge_directory_id -- --data-dir <dir>
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
        _ => Err("usage: migrate_knowledge_directory_id --data-dir <dir>".to_string()),
    }
}

fn main() -> ExitCode {
    match read_data_dir().and_then(|dir| migration::run_migration(&dir)) {
        Ok(outcome) => {
            println!("Set directory_id on {} knowledge doc(s).", outcome.doc_count);
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("Migration stopped: {message}");
            ExitCode::FAILURE
        }
    }
}
