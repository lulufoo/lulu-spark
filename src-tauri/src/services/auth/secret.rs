use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use base64::Engine;

pub(super) fn new_code() -> String {
    let mut bytes = [0u8; 32];
    if !read_random_bytes(&mut bytes) {
        fill_weak_random(&mut bytes);
    }
    URL_SAFE_NO_PAD.encode(bytes)
}

fn read_random_bytes(buf: &mut [u8]) -> bool {
    use std::io::Read;
    #[cfg(unix)]
    {
        if let Ok(mut file) = std::fs::File::open("/dev/urandom") {
            return file.read_exact(buf).is_ok();
        }
    }
    let _ = buf;
    false
}

fn fill_weak_random(buf: &mut [u8]) {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    use std::time::{SystemTime, UNIX_EPOCH};

    for (index, slot) in buf.iter_mut().enumerate() {
        let mut hasher = DefaultHasher::new();
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
            .hash(&mut hasher);
        std::thread::current().id().hash(&mut hasher);
        index.hash(&mut hasher);
        *slot = (hasher.finish() & 0xFF) as u8;
    }
}
