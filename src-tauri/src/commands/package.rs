use crate::host::PackageSnapshot;

#[tauri::command]
pub fn get_package_snapshot() -> PackageSnapshot {
    crate::host::snapshot()
}

#[cfg(test)]
#[path = "../unit-tests/commands/package.rs"]
mod tests;
