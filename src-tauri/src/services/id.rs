//! Shared ID generation (`random_hex12`) for annotation, kb_write, and tag_write.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::time::{SystemTime, UNIX_EPOCH};

pub fn random_hex12() -> String {
    let mut h = DefaultHasher::new();
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
        .hash(&mut h);
    std::thread::current().id().hash(&mut h);
    format!("{:012x}", h.finish() & 0xFFFF_FFFF_FFFFu64)
}

#[cfg(test)]
#[path = "../unit-tests/services/id.rs"]
mod tests;
