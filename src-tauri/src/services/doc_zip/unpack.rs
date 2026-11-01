//! `unpack_doc`: check a package, unzip it to a staging folder, then swap it in for the local files.

use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{self, Cursor, Read};
use std::path::Path;

use zip::ZipArchive;

use crate::services::spark_data::{is_valid_doc_id, DocRow};

use super::install::install;
use super::manifest::{
    check_attached_name, check_member_path, Manifest, ATTACHED_DIR, DIGEST_DIR, MANIFEST_NAME,
    RAW_DIR,
};
use super::MAX_UNPACKED_BYTES;

/// A hidden folder in the workspace folder, on the same disk as the documents so files can move
/// into place by rename. Each unpack works in `{STAGING_DIR}/{doc_id}` and removes it at the end.
const STAGING_DIR: &str = ".zip-staging";

type Archive<'a> = ZipArchive<Cursor<&'a [u8]>>;

/// What a package said about its document, for the caller to write into the document row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Unpacked {
    pub raw_path: String,
    pub digest_path: Option<String>,
    /// Language codes, in the order the package lists them.
    pub translations: Vec<String>,
}

/// Where the body and the digest of a package are to land when that is not where its manifest
/// says (the name is taken by another document of the project). Translations follow the body.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Landing {
    pub raw_path: String,
    pub digest_path: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Manifest,
    Raw,
    Digest,
    Attached,
}

struct Member {
    index: usize,
    kind: Kind,
    /// The path after `raw/`, `digest/` or `attached/`; empty for the manifest.
    rel: String,
}

/// Put the package of document `doc_id` onto disk under `project_id` in the workspace folder
/// `workspace_root`, in place of the files of `existing` (the local row of that document, if it
/// has one): its body, digest, translations and attachment folder all go, so a translation or an
/// attachment the package lacks is gone afterwards.
///
/// Nothing is touched until the whole package has been checked and unzipped. A package that
/// fails any check, or a swap that fails half way, leaves the local files as they were. A
/// package whose files would land on a file of another document is refused.
pub fn unpack_doc(
    workspace_root: &Path,
    project_id: &str,
    doc_id: &str,
    package: &[u8],
    existing: Option<&DocRow>,
) -> Result<Unpacked, String> {
    unpack_capped(workspace_root, project_id, doc_id, package, existing, MAX_UNPACKED_BYTES)
}

/// [`unpack_doc`], with the body and the digest put at `landing` instead of at the paths the
/// manifest names. The attachment folder goes next to the body wherever that is. The result
/// holds the paths the files got.
pub fn unpack_doc_to(
    workspace_root: &Path,
    project_id: &str,
    doc_id: &str,
    package: &[u8],
    existing: Option<&DocRow>,
    landing: &Landing,
) -> Result<Unpacked, String> {
    unpack_inner(workspace_root, project_id, doc_id, package, existing, MAX_UNPACKED_BYTES, Some(landing))
}

/// What the package of `doc_id` says about its document, after the same checks [`unpack_doc`]
/// makes of the package, without touching the disk. Callers use it to pick a [`Landing`].
pub fn peek_package(package: &[u8], doc_id: &str) -> Result<Unpacked, String> {
    if !is_valid_doc_id(doc_id) {
        return Err(format!("invalid doc id {doc_id:?}"));
    }
    let mut archive = ZipArchive::new(Cursor::new(package))
        .map_err(|e| format!("not a zip package: {e}"))?;
    let members = list_members(&mut archive, MAX_UNPACKED_BYTES)?;
    let manifest = read_manifest(&mut archive, &members, doc_id)?;
    check_listed(&members, &manifest)?;
    Ok(Unpacked {
        raw_path: manifest.raw_path,
        digest_path: manifest.digest_path,
        translations: manifest.translations,
    })
}

fn unpack_capped(
    workspace_root: &Path,
    project_id: &str,
    doc_id: &str,
    package: &[u8],
    existing: Option<&DocRow>,
    cap: u64,
) -> Result<Unpacked, String> {
    unpack_inner(workspace_root, project_id, doc_id, package, existing, cap, None)
}

fn unpack_inner(
    workspace_root: &Path,
    project_id: &str,
    doc_id: &str,
    package: &[u8],
    existing: Option<&DocRow>,
    cap: u64,
    landing: Option<&Landing>,
) -> Result<Unpacked, String> {
    if !is_valid_doc_id(doc_id) {
        return Err(format!("invalid doc id {doc_id:?}"));
    }
    if let Some(row) = existing.filter(|row| row.doc_id != doc_id) {
        return Err(format!("local row {} is not document {doc_id}", row.doc_id));
    }
    let mut archive = ZipArchive::new(Cursor::new(package))
        .map_err(|e| format!("not a zip package: {e}"))?;
    let members = list_members(&mut archive, cap)?;
    let manifest = read_manifest(&mut archive, &members, doc_id)?;
    check_listed(&members, &manifest)?;
    let landed = match landing {
        Some(landing) => manifest.landed(&landing.raw_path, landing.digest_path.as_deref())?,
        None => manifest.clone(),
    };

    let staging = workspace_root.join(STAGING_DIR).join(doc_id);
    let _ = fs::remove_dir_all(&staging);
    let result = stage(&mut archive, &members, &staging.join("new"))
        .and_then(|()| install(workspace_root, project_id, &manifest, &landed, &staging, existing));
    let _ = fs::remove_dir_all(&staging);
    let _ = fs::remove_dir(workspace_root.join(STAGING_DIR));
    result?;
    Ok(Unpacked {
        raw_path: landed.raw_path,
        digest_path: landed.digest_path,
        translations: landed.translations,
    })
}

/// Sort every entry into one of the four kinds, by name alone. Anything else is refused:
/// another top-level name, a directory entry, a link, an unsafe path, a name used twice. (The zip
/// reader would silently keep one entry of two with the same name.)
fn list_members(archive: &mut Archive<'_>, cap: u64) -> Result<Vec<Member>, String> {
    if archive.has_duplicate_names() {
        return Err("package uses a file name twice".to_string());
    }
    let mut members = Vec::new();
    let mut declared: u64 = 0;
    for index in 0..archive.len() {
        let entry = archive.by_index(index).map_err(|e| format!("zip entry {index}: {e}"))?;
        let name = entry.name().map_err(|e| format!("zip entry {index}: {e}"))?.into_owned();
        if !entry.is_file() {
            return Err(format!("{name:?} in package is not a plain file"));
        }
        declared = declared.saturating_add(entry.size());
        let (kind, rel) = classify(&name)?;
        members.push(Member { index, kind, rel });
    }
    if declared > cap {
        return Err(format!("package unzips to {declared} bytes, over the {cap} byte limit"));
    }
    let manifests = members.iter().filter(|m| m.kind == Kind::Manifest).count();
    if manifests != 1 {
        return Err(format!("package has {manifests} {MANIFEST_NAME}, expected 1"));
    }
    Ok(members)
}

fn classify(name: &str) -> Result<(Kind, String), String> {
    if name == MANIFEST_NAME {
        return Ok((Kind::Manifest, String::new()));
    }
    if let Some(rel) = name.strip_prefix(&format!("{RAW_DIR}/")) {
        check_member_path(rel)?;
        return Ok((Kind::Raw, rel.to_string()));
    }
    if let Some(rel) = name.strip_prefix(&format!("{DIGEST_DIR}/")) {
        check_member_path(rel)?;
        return Ok((Kind::Digest, rel.to_string()));
    }
    if let Some(file) = name.strip_prefix(&format!("{ATTACHED_DIR}/")) {
        check_attached_name(file)?;
        return Ok((Kind::Attached, file.to_string()));
    }
    Err(format!("unexpected entry in package: {name:?}"))
}

fn read_manifest(
    archive: &mut Archive<'_>,
    members: &[Member],
    doc_id: &str,
) -> Result<Manifest, String> {
    let member = members.iter().find(|m| m.kind == Kind::Manifest).ok_or("no manifest")?;
    let mut bytes = Vec::new();
    archive
        .by_index(member.index)
        .and_then(|mut entry| entry.read_to_end(&mut bytes).map_err(Into::into))
        .map_err(|e| format!("{MANIFEST_NAME}: {e}"))?;
    Manifest::parse(&bytes, doc_id)
}

/// The manifest lists the files under `raw/` and `digest/`; the package holds those and no others.
/// Attachments are not listed: the folder is whatever `attached/` holds.
fn check_listed(members: &[Member], manifest: &Manifest) -> Result<(), String> {
    let digest: Vec<String> = manifest.digest_path.iter().cloned().collect();
    for (kind, dir, listed) in [
        (Kind::Raw, RAW_DIR, manifest.raw_rels()),
        (Kind::Digest, DIGEST_DIR, digest),
    ] {
        let held: HashSet<&str> =
            members.iter().filter(|m| m.kind == kind).map(|m| m.rel.as_str()).collect();
        if let Some(rel) = listed.iter().find(|rel| !held.contains(rel.as_str())) {
            return Err(format!("{MANIFEST_NAME} lists {dir}/{rel}, but the package has no such file"));
        }
        if let Some(rel) = held.iter().find(|rel| !listed.iter().any(|l| l == **rel)) {
            return Err(format!("package has {dir}/{rel}, which {MANIFEST_NAME} does not list"));
        }
    }
    Ok(())
}

/// Unzip every file except the manifest into `new_dir`, as `raw/…`, `digest/…`, `attached/…`.
/// The zip reader stops a file at the size its header declares, and `list_members` has capped
/// the sum of those sizes, so this cannot write more than the cap.
fn stage(archive: &mut Archive<'_>, members: &[Member], new_dir: &Path) -> Result<(), String> {
    for member in members.iter().filter(|m| m.kind != Kind::Manifest) {
        let dir = match member.kind {
            Kind::Raw => RAW_DIR,
            Kind::Digest => DIGEST_DIR,
            _ => ATTACHED_DIR,
        };
        let target = new_dir.join(dir).join(&member.rel);
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        let mut out = File::create(&target).map_err(|e| format!("{}: {e}", target.display()))?;
        let mut entry = archive
            .by_index(member.index)
            .map_err(|e| format!("{dir}/{}: {e}", member.rel))?;
        io::copy(&mut entry, &mut out).map_err(|e| format!("{dir}/{}: {e}", member.rel))?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "../../unit-tests/services/doc_zip_unpack.rs"]
mod tests;
