//! Shared ID generation (`random_hex12`) for annotation, corpus write, and tag_write.

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

/// 32-char lowercase hex entry id (`secrets.token_hex(16)` equivalent).
pub fn random_entry_id() -> String {
    let mut bytes = [0u8; 16];
    if read_random_bytes(&mut bytes) {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    } else {
        let mut out = random_hex12();
        out.push_str(&random_hex12());
        out.push_str(&random_hex12().chars().take(8).collect::<String>());
        out
    }
}

fn read_random_bytes(buf: &mut [u8]) -> bool {
    use std::io::Read;
    #[cfg(unix)]
    {
        if let Ok(mut f) = std::fs::File::open("/dev/urandom") {
            return f.read_exact(buf).is_ok();
        }
    }
    let _ = buf;
    false
}

#[cfg(test)]
#[path = "../unit-tests/services/id.rs"]
mod tests;
