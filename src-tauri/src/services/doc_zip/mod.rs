//! One document as one zip, for cloud sync. No network, no database, no version number.
//!
//! The package is named by the 32-hex doc id and holds every file of the document:
//!
//! ```text
//! manifest.json            {"format":1,"doc_id":…,"raw_path":…,"digest_path":…|null,"translations":["en"]}
//! raw/<raw_path>           the body
//! raw/<stem>-<lang>.md     one per language in `translations`
//! digest/<digest_path>     only when `digest_path` is not null
//! attached/<name>          the files of `.attached/{doc_id}/`, flat
//! ```
//!
//! A document with only a body is packed the same way. [`pack_doc`] reads the files
//! of a local document. [`unpack_doc`] puts a package onto disk in place of them.

mod install;
mod manifest;
mod pack;
mod unpack;

use serde_json::Value;

pub use pack::pack_doc;
pub use unpack::{peek_package, unpack_doc, unpack_doc_to, Landing, Unpacked};

/// The files of a package may add up to this many bytes once unzipped. A real document stays far
/// below it; the limit stops a small package that inflates into gigabytes.
pub const MAX_UNPACKED_BYTES: u64 = 100 * 1024 * 1024;

/// The message of an `{ "error": … }` value from the docs helpers.
fn error_text(value: Value) -> String {
    value.get("error").and_then(Value::as_str).unwrap_or("invalid path").to_string()
}

#[cfg(test)]
#[path = "../../unit-tests/services/doc_zip.rs"]
mod tests;
