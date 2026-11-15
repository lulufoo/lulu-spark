use crate::services::board_sketch::{diagram_id, failure, project_sketch, BoardDocument, BoardSketchReply};
use super::{attach, Coordinator};

impl Coordinator {
    pub(crate) fn read_cached(&self, url: &str) -> BoardSketchReply {
        let id = match diagram_id(url) { Ok(id) => id, Err(status) => return failure(status) };
        let mut events = Vec::new();
        let result = {
            let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            match self.load_credentials(&mut state, &mut events) {
                Err(failure) => failure,
                Ok(None) => failure("unconfigured"),
                Ok(Some(credentials)) => {
                    let identity = state.get_identity(&credentials.cache_scope, &id);
                    if state.revoked {
                        attach(failure("unauthorized"), identity)
                    } else if state.denied.contains(&id) {
                        attach(failure("forbidden"), identity)
                    } else {
                        let record = self.record_cache_error("read", self.cache.read(url, &credentials.cache_scope))
                            .flatten();
                        let projection = match record {
                            Some(record) if record.version <= i32::MAX as u32 && record.diagram_id == id => {
                                project_sketch(&BoardDocument { bmd: record.bmd, title: record.title,
                                    version: record.version as i32 })
                            }
                            Some(_) => {
                                self.record_cache_error("remove", self.cache.remove_url(url));
                                failure("cache_miss")
                            }
                            None => failure("cache_miss"),
                        };
                        attach(projection, identity)
                    }
                }
            }
        };
        self.emit(events);
        result
    }

    pub(crate) fn set_pat(&self, value: &str) -> Result<(), String> {
        self.change_credentials(Some(value))
    }

    pub(crate) fn delete_pat(&self) -> Result<(), String> { self.change_credentials(None) }

    fn change_credentials(&self, value: Option<&str>) -> Result<(), String> {
        let event = {
            let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            let previous = state.scope.clone().or_else(|| {
                self.credentials.get().ok().flatten().map(|v| v.cache_scope)
            });
            match value {
                Some(value) => self.credentials.set(value)?,
                None => self.credentials.delete()?,
            }
            // A successful durable write retires the previous generation even
            // when a subsequent Keychain read cannot obtain the new identity.
            let next = self.credentials.get().ok().flatten().map(|v| v.cache_scope);
            state.advance_scope(next);
            self.record_cache_error("clear", self.cache.clear());
            previous.map(|scope| state.build_credential_event(&scope,
                if value.is_some() { "credential_changed" } else { "credential_removed" }))
        };
        self.emit(event.into_iter().collect());
        Ok(())
    }
}
