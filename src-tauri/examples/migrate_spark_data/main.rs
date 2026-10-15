//! One-off tool: import the old notes and knowledge metadata files into
//! `spark-data.sqlite`, prove the import matches, then delete the old files.
//!
//! Run by hand, once. The app never calls it and never reads the old files.
//!
//!   cargo run --example migrate_spark_data -- --data-dir <dir>
//!
//! `<dir>` is the app data directory: it holds `notes/` and `knowledge/`, and receives
//! `spark-data.sqlite`. Try it on a copy first. Back up `<dir>` first: afterwards the
//! database is the only copy.

mod apply;
mod cleanup;
mod migration;
mod old_data;
mod plan;
mod reconcile;
mod restore;
mod shapes;
#[cfg(test)]
mod tests;

use std::path::PathBuf;
use std::process::ExitCode;

fn read_data_dir() -> Result<PathBuf, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.as_slice() {
        [flag, dir] if flag == "--data-dir" => Ok(PathBuf::from(dir)),
        _ => Err("usage: migrate_spark_data --data-dir <dir>".to_string()),
    }
}

fn main() -> ExitCode {
    let result = read_data_dir().and_then(|dir| migration::run_migration(&dir));
    match result {
        Ok(outcome) => {
            println!("Reconciled, all identical: {}", outcome.report);
            println!("Deleted {} old file(s).", outcome.files_deleted);
            if outcome.dropped_fields > 0 {
                println!(
                    "{} index entr(ies) carried fields the new model has no column for; \
                     they are saved in migration-dropped-fields.json.",
                    outcome.dropped_fields
                );
            }
            ExitCode::SUCCESS
        }
        Err(message) => {
            eprintln!("Migration stopped: {message}");
            ExitCode::FAILURE
        }
    }
}
