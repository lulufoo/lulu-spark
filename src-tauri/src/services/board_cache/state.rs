use std::collections::{BTreeSet, HashMap};
use std::sync::Arc;

use crate::config::vault::BoardCredentialSnapshot;
use crate::services::board_sketch::{failure, BoardReadIdentity, BoardSketchReply};

use super::{BoardCacheInvalidated, Coordinator, Flight};

pub(super) struct State {
    pub initialized: bool,
    pub scope: Option<String>,
    pub generation: u64,
    pub diagram_generations: HashMap<String, u64>,
    pub revoked: bool,
    pub denied: BTreeSet<String>,
    pub flights: HashMap<(String, String), Arc<Flight>>,
}

impl Default for State {
    fn default() -> Self {
        Self { initialized: false, scope: None, generation: 1,
            diagram_generations: HashMap::new(), revoked: false,
            denied: BTreeSet::new(), flights: HashMap::new() }
    }
}

impl State {
    pub fn get_identity(&self, scope: &str, diagram_id: &str) -> BoardReadIdentity {
        BoardReadIdentity { credential_scope: scope.into(), generation: self.generation,
            diagram_id: diagram_id.into(),
            diagram_generation: self.diagram_generations.get(diagram_id).copied().unwrap_or(0) }
    }

    pub fn matches(&self, identity: &BoardReadIdentity) -> bool {
        self.scope.as_deref() == Some(identity.credential_scope.as_str())
            && self.generation == identity.generation
            && self.diagram_generations.get(&identity.diagram_id).copied().unwrap_or(0)
                == identity.diagram_generation
    }

    pub fn advance_scope(&mut self, scope: Option<String>) {
        self.cancel_all();
        self.initialized = true;
        self.scope = scope;
        self.generation += 1;
        self.diagram_generations.clear();
        self.revoked = false;
        self.denied.clear();
    }

    pub fn cancel_all(&mut self) {
        for (_, flight) in self.flights.drain() { flight.cancel(); }
    }

    pub fn invalidate_diagram(&mut self, id: &str) {
        *self.diagram_generations.entry(id.into()).or_default() += 1;
        self.denied.insert(id.into());
        if let Some(scope) = &self.scope {
            if let Some(flight) = self.flights.remove(&(scope.clone(), id.into())) { flight.cancel(); }
        }
    }

    pub fn start_diagram_read(&mut self, id: &str) {
        *self.diagram_generations.entry(id.into()).or_default() += 1;
    }

    pub fn build_credential_event(&self, scope: &str, reason: &'static str) -> BoardCacheInvalidated {
        BoardCacheInvalidated { credential_scope: scope.into(), generation: self.generation,
            diagram_id: None, diagram_generation: None, reason }
    }

    pub fn build_diagram_event(&self, scope: &str, id: &str, reason: &'static str) -> BoardCacheInvalidated {
        BoardCacheInvalidated { credential_scope: scope.into(), generation: self.generation,
            diagram_id: Some(id.into()),
            diagram_generation: Some(self.diagram_generations.get(id).copied().unwrap_or(0)), reason }
    }
}

impl Coordinator {
    pub(super) fn load_credentials(
        &self, state: &mut State, events: &mut Vec<BoardCacheInvalidated>,
    ) -> Result<Option<BoardCredentialSnapshot>, BoardSketchReply> {
        let value = self.credentials.get().map_err(|_| failure("unavailable"))?;
        let scope = value.as_ref().map(|v| v.cache_scope.clone());
        if !state.initialized {
            state.initialized = true;
            state.scope = scope;
        } else if state.scope != scope {
            let previous = state.scope.clone();
            let reason = if scope.is_some() { "credential_changed" } else { "credential_removed" };
            state.advance_scope(scope);
            self.record_cache_error("clear", self.cache.clear());
            if let Some(previous) = previous { events.push(state.build_credential_event(&previous, reason)); }
        }
        if let Some(value) = &value {
            state.revoked |= value.revoked;
            state.denied.extend(value.denied_diagrams.iter().cloned());
        }
        Ok(value)
    }
}
