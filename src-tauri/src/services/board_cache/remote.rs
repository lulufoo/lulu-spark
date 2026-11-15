use std::sync::Arc;
use std::time::Instant;

use crate::config::vault::BoardCredentialSnapshot;
use crate::repositories::board_cache::BoardCacheRecord;
use crate::services::board_sketch::{diagram_id, failure, log_read, project_sketch, BoardReadIdentity,
    BoardSketchReply, DocumentFetch};
use super::{attach, BoardCacheInvalidated, CompletedRead, Coordinator, Flight};

impl Coordinator {
    pub(crate) fn read_remote(&self, url: &str) -> BoardSketchReply {
        let started = Instant::now();
        let id = match diagram_id(url) { Ok(id) => id, Err(status) => return failure(status) };
        let vault_at = Instant::now();
        let mut events = Vec::new();
        let prepared = {
            let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            match self.load_credentials(&mut state, &mut events) {
                Err(error) => Err(error),
                Ok(None) => Err(failure("unconfigured")),
                Ok(Some(credentials)) => {
                    let identity = state.get_identity(&credentials.cache_scope, &id);
                    if state.revoked {
                        Err(attach(failure("unauthorized"), identity))
                    } else {
                        let key = (credentials.cache_scope.clone(), id.clone());
                        if let Some(flight) = state.flights.get(&key) {
                            Ok((credentials, flight.clone(), false))
                        } else {
                            state.start_diagram_read(&id);
                            let identity = state.get_identity(&credentials.cache_scope, &id);
                            let flight = Arc::new(Flight::new(identity));
                            state.flights.insert(key, flight.clone());
                            Ok((credentials, flight, true))
                        }
                    }
                }
            }
        };
        self.emit(events);
        let vault_ms = vault_at.elapsed();
        let (credentials, flight, leader) = match prepared {
            Ok(prepared) => prepared,
            Err(reply) => {
                log_read(&reply, &id, started.elapsed(), Some(vault_ms), None, None);
                return reply;
            }
        };
        let fetch_at = Instant::now();
        if leader {
            let fetched = self.fetcher.fetch(&credentials.pat, &id);
            self.finish(&flight, fetched);
        }
        // Neither the network call nor this Condvar wait holds coordinator state.
        let completed = flight.wait();
        let fetch_ms = fetch_at.elapsed();
        let reply = self.commit_alias(url, &completed);
        log_read(&reply, &id, started.elapsed(), Some(vault_ms), Some(fetch_ms), completed.bytes);
        reply
    }

    fn finish(&self, flight: &Arc<Flight>, fetched: DocumentFetch) {
        let mut events = Vec::new();
        {
            let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            let current = self.load_credentials(&mut state, &mut events);
            let key = (flight.identity.credential_scope.clone(), flight.identity.diagram_id.clone());
            if state.flights.get(&key).is_some_and(|active| Arc::ptr_eq(active, flight)) {
                state.flights.remove(&key);
            }
            let result = match current {
                Ok(Some(credentials)) if state.matches(&flight.identity) && !state.revoked => {
                    self.apply_remote(&mut state, &credentials, &flight.identity, fetched.result, &mut events)
                }
                Err(error) if state.matches(&flight.identity) => {
                    self.keep_denial(&state, &flight.identity, error, &mut events)
                }
                _ => error_read(failure("superseded"), &flight.identity),
            };
            flight.complete(CompletedRead { bytes: fetched.bytes, ..result });
        }
        self.emit(events);
    }

    fn apply_remote(
        &self, state: &mut super::state::State, credentials: &BoardCredentialSnapshot,
        identity: &BoardReadIdentity,
        result: Result<super::BoardDocument, BoardSketchReply>,
        events: &mut Vec<BoardCacheInvalidated>,
    ) -> CompletedRead {
        let scope = &credentials.cache_scope;
        let id = &identity.diagram_id;
        match result {
            Ok(document) => {
                if state.denied.contains(id) {
                    match self.credentials.deny(scope, id, false) {
                        Ok(false) => return error_read(failure("superseded"), identity),
                        Ok(true) => { state.denied.remove(id); }
                        Err(_) => {
                            super::super::board_sketch_log::cache_failure("access_persist");
                        }
                    }
                }
                CompletedRead { result: Ok(document), identity: identity.clone(), bytes: None }
            }
            Err(error) if error.status == "unauthorized" => {
                match self.credentials.revoke(scope) {
                    Ok(false) => return error_read(failure("superseded"), identity),
                    Ok(true) => {}
                    Err(_) => super::super::board_sketch_log::cache_failure("access_persist"),
                }
                state.advance_scope(Some(scope.clone()));
                state.revoked = true;
                self.record_cache_error("remove_scope", self.cache.remove_scope(scope));
                events.push(state.build_credential_event(scope, "unauthorized"));
                error_read(error, &state.get_identity(scope, id))
            }
            Err(error) if matches!(error.status, "forbidden" | "not_found") => {
                match self.credentials.deny(scope, id, true) {
                    Ok(false) => return error_read(failure("superseded"), identity),
                    Ok(true) => {}
                    Err(_) => super::super::board_sketch_log::cache_failure("access_persist"),
                }
                state.invalidate_diagram(id);
                self.record_cache_error("remove_diagram", self.cache.remove_diagram(id, scope));
                events.push(state.build_diagram_event(scope, id, error.status));
                error_read(error, &state.get_identity(scope, id))
            }
            Err(error) => self.keep_denial(state, identity, error, events),
        }
    }

    fn keep_denial(
        &self, state: &super::state::State, identity: &BoardReadIdentity,
        error: BoardSketchReply, events: &mut Vec<BoardCacheInvalidated>,
    ) -> CompletedRead {
        if state.denied.contains(&identity.diagram_id) {
            events.push(state.build_diagram_event(&identity.credential_scope,
                &identity.diagram_id, "forbidden"));
            error_read(failure("forbidden"), identity)
        } else {
            error_read(error, identity)
        }
    }

    fn commit_alias(&self, url: &str, completed: &CompletedRead) -> BoardSketchReply {
        let mut events = Vec::new();
        let reply = {
            let mut state = self.state.lock().unwrap_or_else(|e| e.into_inner());
            let current = self.load_credentials(&mut state, &mut events);
            match current {
                Ok(Some(credentials)) if state.matches(&completed.identity) => {
                    let document = match &completed.result {
                        Err(error) => return attach(error.clone(), completed.identity.clone()),
                        Ok(document) if !state.revoked => document,
                        Ok(_) => return attach(failure("superseded"), completed.identity.clone()),
                    };
                    if !state.denied.contains(&completed.identity.diagram_id)
                        && !credentials.denied_diagrams.contains(&completed.identity.diagram_id)
                    {
                        let existing = self.record_cache_error("read", self.cache.read(url, &credentials.cache_scope))
                            .flatten();
                        let unchanged = existing.is_some_and(|record| record.version == document.version as u32
                            && record.diagram_id == completed.identity.diagram_id);
                        if !unchanged {
                            let record = BoardCacheRecord::new(url.into(), completed.identity.diagram_id.clone(),
                                document.version as u32, document.bmd.clone(), document.title.clone(),
                                credentials.cache_scope);
                            // Check and store under the same lock as invalidation.
                            self.record_cache_error("store", self.cache.store(&record));
                        }
                    }
                    attach(project_sketch(document), completed.identity.clone())
                }
                Err(error) if state.matches(&completed.identity) => {
                    attach(error, completed.identity.clone())
                }
                _ => attach(failure("superseded"), completed.identity.clone()),
            }
        };
        self.emit(events);
        reply
    }
}

fn error_read(error: BoardSketchReply, identity: &BoardReadIdentity) -> CompletedRead {
    CompletedRead { result: Err(error), identity: identity.clone(), bytes: None }
}

#[cfg(test)]
#[path = "../../unit-tests/services/board_cache/remote.rs"]
mod tests;
