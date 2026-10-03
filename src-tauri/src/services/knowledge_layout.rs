//! Knowledge registry lives at `{spark_root}/knowledge/`.
//! One-time move from `{spark_root}/sediment-kb/`.

use std::fs;
use std::path::Path;

/// Ensure `{wb}/knowledge` exists, moving `sediment-kb` if that is still the on-disk name.
pub fn ensure_knowledge_registry_layout(wb: &Path) -> Result<(), String> {
    let dest = wb.join("knowledge");
    if dest.is_dir() {
        return Ok(());
    }
    let src = wb.join("sediment-kb");
    if src.is_dir() {
        fs::rename(&src, &dest).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "../unit-tests/services/knowledge_layout.rs"]
mod tests;
