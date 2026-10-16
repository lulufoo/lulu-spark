//! One run: read, plan, write in one transaction, reconcile, then delete.

use std::fs;
use std::path::Path;

use lulu_spark_lib::services::spark_data::open_at;

use crate::apply::write_plan;
use crate::cleanup::{delete_files, prune_empty_dirs};
use crate::old_data::read_old_data;
use crate::plan::build_plan;
use crate::reconcile::{reconcile, Report};

const MAX_DIFFS_SHOWN: usize = 40;

pub struct Outcome {
    pub report: Report,
    pub files_deleted: usize,
    pub dropped_fields: usize,
}

fn format_diffs(diffs: &[String]) -> String {
    let shown: Vec<&str> = diffs.iter().take(MAX_DIFFS_SHOWN).map(String::as_str).collect();
    let more = diffs.len().saturating_sub(MAX_DIFFS_SHOWN);
    let tail = if more > 0 { format!("\n... and {more} more") } else { String::new() };
    format!("{} difference(s); nothing was written or deleted:\n{}{tail}", diffs.len(), shown.join("\n"))
}

/// Import the old notes and knowledge files under `data_dir` into
/// `data_dir/spark-data.sqlite`. The database write and the reconciliation share one
/// transaction: any difference rolls it back and no file is deleted.
pub fn run_migration(data_dir: &Path) -> Result<Outcome, String> {
    let old = read_old_data(data_dir)?;
    let files = old.files_read();
    if files.is_empty() {
        return Err(format!("no old metadata files under {}", data_dir.display()));
    }
    let plan = build_plan(&old)?;
    let conn = open_at(&data_dir.join("spark-data.sqlite"))?;
    let tx = conn.unchecked_transaction().map_err(|e| e.to_string())?;
    write_plan(&tx, &plan)?;
    let reconciled = reconcile(&old, &plan, &tx)?;
    if !reconciled.diffs.is_empty() {
        return Err(format_diffs(&reconciled.diffs));
    }
    if !plan.dropped.is_empty() {
        let text = serde_json::to_string_pretty(&plan.dropped).map_err(|e| e.to_string())?;
        fs::write(data_dir.join("migration-dropped-fields.json"), text).map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    delete_files(&files)?;
    old.dirs_to_prune().iter().for_each(|dir| prune_empty_dirs(dir));
    Ok(Outcome { report: reconciled.report, files_deleted: files.len(), dropped_fields: plan.dropped.len() })
}
