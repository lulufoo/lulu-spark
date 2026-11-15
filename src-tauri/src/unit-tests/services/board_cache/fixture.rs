use std::collections::BTreeSet;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use crate::repositories::board_cache::BoardCache;
use super::*;

pub const URL: &str = "https://luluboard.app/#b:b_0123abcd";
pub const ALIAS: &str = "https://luluboard.app/view?utm=1#b:b_0123abcd";
pub const OTHER: &str = "https://luluboard.app/#b:b_89abcdef";
pub const ID: &str = "b_0123abcd";
pub const SCOPE: &str = "scope-a";

pub fn document(version: i32, bmd: &str) -> DocumentFetch {
    DocumentFetch { result: Ok(BoardDocument { bmd: bmd.into(), title: "Title".into(), version }), bytes: Some(40) }
}

pub fn sketch(version: i32) -> DocumentFetch { document(version, "sketch\ncontent PGgxPkhpPC9oMT4=") }
pub fn error(status: &'static str) -> DocumentFetch {
    DocumentFetch { result: Err(failure(status)), bytes: None }
}

pub struct Credentials {
    pub current: Mutex<Option<BoardCredentialSnapshot>>,
    pub fail_write: AtomicBool,
    next: AtomicUsize,
}
impl Credentials {
    fn new() -> Self {
        Self { current: Mutex::new(Some(BoardCredentialSnapshot { pat: "secret-a".into(),
            cache_scope: SCOPE.into(), revoked: false, denied_diagrams: BTreeSet::new() })),
            fail_write: AtomicBool::new(false), next: AtomicUsize::new(1) }
    }
    fn ensure_writable(&self) -> Result<(), String> {
        if self.fail_write.load(Ordering::SeqCst) { Err("credential persistence unavailable".into()) }
        else { Ok(()) }
    }
}
impl CredentialStore for Credentials {
    fn get(&self) -> Result<Option<BoardCredentialSnapshot>, String> {
        Ok(self.current.lock().unwrap().clone())
    }
    fn set(&self, value: &str) -> Result<(), String> {
        self.ensure_writable()?;
        if value.trim().is_empty() { return Err("invalid board pat".into()); }
        let next = self.next.fetch_add(1, Ordering::SeqCst);
        *self.current.lock().unwrap() = Some(BoardCredentialSnapshot { pat: value.trim().into(),
            cache_scope: format!("scope-new-{next}"), revoked: false, denied_diagrams: BTreeSet::new() });
        Ok(())
    }
    fn delete(&self) -> Result<(), String> { self.ensure_writable()?; *self.current.lock().unwrap()=None; Ok(()) }
    fn revoke(&self, scope: &str) -> Result<bool, String> {
        self.ensure_writable()?;
        let mut value = self.current.lock().unwrap();
        match value.as_mut() {
            Some(value) if value.cache_scope == scope => { value.revoked=true; Ok(true) }
            _ => Ok(false),
        }
    }
    fn deny(&self, scope: &str, id: &str, denied: bool) -> Result<bool, String> {
        self.ensure_writable()?;
        let mut value = self.current.lock().unwrap();
        match value.as_mut() {
            Some(value) if value.cache_scope == scope => {
                if denied { value.denied_diagrams.insert(id.into()); }
                else { value.denied_diagrams.remove(id); }
                Ok(true)
            }
            _ => Ok(false),
        }
    }
}

pub struct Cache {
    pub disk: BoardCache,
    pub fail_delete: AtomicBool,
    pub fail_store: AtomicBool,
    pub stores: AtomicUsize,
    pub on_store: Mutex<Option<Arc<dyn Fn() + Send + Sync>>>,
}
impl SourceCache for Cache {
    fn read(&self, url: &str, scope: &str) -> Result<Option<BoardCacheRecord>, String> {
        self.disk.read(url, scope)
    }
    fn store(&self, value: &BoardCacheRecord) -> Result<StoreOutcome, String> {
        if let Some(callback)=self.on_store.lock().unwrap().clone() { callback(); }
        self.stores.fetch_add(1, Ordering::SeqCst);
        if self.fail_store.load(Ordering::SeqCst) { Err("cache write unavailable".into()) }
        else { self.disk.store(value) }
    }
    fn remove_url(&self, url: &str) -> Result<(), String> {
        if self.fail_delete.load(Ordering::SeqCst) { Err("remove unavailable".into()) }
        else { self.disk.remove_url(url) }
    }
    fn remove_diagram(&self, id: &str, scope: &str) -> Result<usize, String> {
        if self.fail_delete.load(Ordering::SeqCst) { Err("remove unavailable".into()) }
        else { self.disk.remove_diagram(id, scope) }
    }
    fn remove_scope(&self, scope: &str) -> Result<usize, String> {
        if self.fail_delete.load(Ordering::SeqCst) { Err("remove unavailable".into()) }
        else { self.disk.remove_scope(scope) }
    }
    fn clear(&self) -> Result<(), String> {
        if self.fail_delete.load(Ordering::SeqCst) { Err("remove unavailable".into()) }
        else { self.disk.clear() }
    }
}

pub struct Fetch {
    pub calls: AtomicUsize,
    pub run: Box<dyn Fn(&str, &str) -> DocumentFetch + Send + Sync>,
}
impl Fetcher for Fetch {
    fn fetch(&self, secret: &str, id: &str) -> DocumentFetch {
        self.calls.fetch_add(1, Ordering::SeqCst);
        (self.run)(secret, id)
    }
}

pub struct Harness {
    pub _dir: tempfile::TempDir,
    pub credentials: Arc<Credentials>,
    pub cache: Arc<Cache>,
    pub fetch: Arc<Fetch>,
    pub service: Arc<Coordinator>,
    pub events: Arc<Mutex<Vec<BoardCacheInvalidated>>>,
}
impl Harness {
    pub fn new(run: impl Fn(&str, &str) -> DocumentFetch + Send + Sync + 'static) -> Self {
        let dir = tempfile::tempdir().unwrap();
        let credentials = Arc::new(Credentials::new());
        let cache = Arc::new(Cache { disk: BoardCache::new(dir.path()), fail_delete: AtomicBool::new(false),
            fail_store: AtomicBool::new(false), stores: AtomicUsize::new(0), on_store: Mutex::new(None) });
        let fetch = Arc::new(Fetch { calls: AtomicUsize::new(0), run: Box::new(run) });
        let service = Arc::new(Coordinator::new(credentials.clone(), cache.clone(), fetch.clone()));
        let events = Arc::new(Mutex::new(Vec::new()));
        let sink=events.clone();
        *service.handler.lock().unwrap()=Some(Arc::new(move |event|sink.lock().unwrap().push(event)));
        Self { _dir: dir, credentials, cache, fetch, service, events }
    }
    pub fn restart(&self) -> Coordinator {
        Coordinator::new(self.credentials.clone(), self.cache.clone(), self.fetch.clone())
    }
    pub fn seed(&self, url: &str, id: &str, version: u32) {
        self.cache.disk.store(&BoardCacheRecord::new(url.into(), id.into(), version,
            "sketch\ncontent PGgxPkhpPC9oMT4=".into(), "Title".into(), SCOPE.into())).unwrap();
    }
}
