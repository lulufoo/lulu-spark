//! Read the old metadata files. Nothing here writes or deletes.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

/// One old annotation JSON file, addressed by its path under `annotations/` or
/// `.knowledge_annotations/` (for example `topic/note.json`).
pub struct OldFile {
    pub file: PathBuf,
    pub value: Value,
}

pub struct OldNotes {
    pub index_file: PathBuf,
    pub entries: BTreeMap<String, Value>,
    pub annotations: BTreeMap<String, OldFile>,
    pub registry_file: Option<PathBuf>,
    pub registry: Value,
}

pub struct OldKnowledgeRepo {
    pub repo: String,
    pub dir: PathBuf,
    pub annotations: BTreeMap<String, OldFile>,
}

#[derive(Default)]
pub struct OldData {
    pub notes: Option<OldNotes>,
    pub knowledge: Vec<OldKnowledgeRepo>,
}

pub fn read_json(path: &Path) -> Result<Value, String> {
    let text = fs::read_to_string(path).map_err(|e| format!("{}: {e}", path.display()))?;
    serde_json::from_str(&text).map_err(|e| format!("{}: {e}", path.display()))
}

fn collect_json_files(base: &Path, dir: &Path, out: &mut BTreeMap<String, OldFile>) -> Result<(), String> {
    let entries = fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?;
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_json_files(base, &path, out)?;
        } else if path.extension().is_some_and(|ext| ext == "json") {
            let rel = path.strip_prefix(base).map_err(|e| e.to_string())?;
            let key = rel.to_string_lossy().replace(std::path::MAIN_SEPARATOR, "/");
            out.insert(key, OldFile { value: read_json(&path)?, file: path });
        }
    }
    Ok(())
}

fn read_annotations(dir: &Path) -> Result<BTreeMap<String, OldFile>, String> {
    let mut out = BTreeMap::new();
    if dir.is_dir() {
        collect_json_files(dir, dir, &mut out)?;
    }
    Ok(out)
}

fn read_notes(notes_dir: &Path) -> Result<Option<OldNotes>, String> {
    let index_file = notes_dir.join("index.json");
    if !index_file.is_file() {
        return Ok(None);
    }
    let index = read_json(&index_file)?;
    let entries = index
        .get("entries")
        .and_then(Value::as_object)
        .ok_or_else(|| format!("{}: `entries` must be an object keyed by id", index_file.display()))?
        .iter()
        .map(|(id, entry)| (id.clone(), entry.clone()))
        .collect();
    let registry_path = notes_dir.join("tags").join("registry.json");
    let (registry_file, registry) = if registry_path.is_file() {
        (Some(registry_path.clone()), read_json(&registry_path)?)
    } else {
        (None, serde_json::json!({ "keys": {} }))
    };
    Ok(Some(OldNotes {
        index_file,
        entries,
        annotations: read_annotations(&notes_dir.join("annotations"))?,
        registry_file,
        registry,
    }))
}

fn read_knowledge(knowledge_dir: &Path) -> Result<Vec<OldKnowledgeRepo>, String> {
    let mut repos = Vec::new();
    let Ok(entries) = fs::read_dir(knowledge_dir) else {
        return Ok(repos);
    };
    for entry in entries.flatten() {
        let dir = entry.path();
        let ann_dir = dir.join(".knowledge_annotations");
        if !ann_dir.is_dir() {
            continue;
        }
        let annotations = read_annotations(&ann_dir)?;
        if annotations.is_empty() {
            continue;
        }
        repos.push(OldKnowledgeRepo {
            repo: entry.file_name().to_string_lossy().into_owned(),
            dir,
            annotations,
        });
    }
    repos.sort_by(|a, b| a.repo.cmp(&b.repo));
    Ok(repos)
}

/// `data_dir` holds `notes/` and `knowledge/`, the same layout the app uses.
pub fn read_old_data(data_dir: &Path) -> Result<OldData, String> {
    Ok(OldData {
        notes: read_notes(&data_dir.join("notes"))?,
        knowledge: read_knowledge(&data_dir.join("knowledge"))?,
    })
}

impl OldData {
    /// Folders that held only the files above; removed once they are empty.
    pub fn dirs_to_prune(&self) -> Vec<PathBuf> {
        let mut dirs = Vec::new();
        if let Some(notes) = &self.notes {
            if let Some(notes_dir) = notes.index_file.parent() {
                dirs.push(notes_dir.join("annotations"));
                dirs.push(notes_dir.join("tags"));
            }
        }
        dirs.extend(self.knowledge.iter().map(|r| r.dir.join(".knowledge_annotations")));
        dirs
    }

    /// Every file the tool read, which is what gets deleted after reconciliation.
    pub fn files_read(&self) -> Vec<PathBuf> {
        let mut files = Vec::new();
        if let Some(notes) = &self.notes {
            files.push(notes.index_file.clone());
            files.extend(notes.registry_file.clone());
            files.extend(notes.annotations.values().map(|f| f.file.clone()));
        }
        for repo in &self.knowledge {
            files.extend(repo.annotations.values().map(|f| f.file.clone()));
        }
        files
    }
}
