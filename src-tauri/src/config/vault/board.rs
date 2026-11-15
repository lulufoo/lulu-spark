//! Board credential snapshots and durable, opaque cache identities.

use std::collections::BTreeSet;

use rand::{rngs::OsRng, RngCore};

use super::store::{io_lock, load_raw, store_raw, update_vault};
use super::types::{BoardSecrets, SecretError, Vault};

#[derive(Clone, PartialEq, Eq)]
pub struct BoardCredentialSnapshot {
    pub pat: String,
    pub cache_scope: String,
    pub revoked: bool,
    pub denied_diagrams: BTreeSet<String>,
}

impl std::fmt::Debug for BoardCredentialSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("BoardCredentialSnapshot")
            .field("pat", &"[redacted]")
            .field("cache_scope", &self.cache_scope)
            .field("revoked", &self.revoked)
            .field("denied_diagrams", &self.denied_diagrams)
            .finish()
    }
}

fn new_cache_scope() -> Result<String, SecretError> {
    let mut bytes = [0_u8; 16];
    OsRng.try_fill_bytes(&mut bytes)
        .map_err(|_| SecretError::Store("Board cache scope unavailable".into()))?;
    Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
}

/// Legacy credentials gain a scope only after that scope has been persisted.
pub fn get_board_credentials() -> Result<Option<BoardCredentialSnapshot>, SecretError> {
    let _guard = io_lock();
    ensure_board_credentials(&mut load_raw()?, store_raw)
}

fn ensure_board_credentials(
    vault: &mut Vault,
    persist: impl FnOnce(&Vault) -> Result<(), SecretError>,
) -> Result<Option<BoardCredentialSnapshot>, SecretError> {
    let Some(pat) = vault.board_pat().map(str::to_string) else {
        return Ok(None);
    };
    let board = vault.board.as_mut().expect("Board PAT has a Board secret tree");
    if board.cache_scope.as_deref().is_none_or(|scope| scope.trim().is_empty()) {
        board.cache_scope = Some(new_cache_scope()?);
        persist(vault)?;
    }
    let board = vault.board.as_ref().expect("Board secret tree remains present");
    Ok(Some(BoardCredentialSnapshot {
        pat,
        cache_scope: board.cache_scope.clone().expect("Board scope is persisted"),
        revoked: board.revoked,
        denied_diagrams: board.denied_diagrams.clone(),
    }))
}

/// Every explicit save retires the old cache identity, even for the same PAT.
pub fn set_board_pat(value: &str) -> Result<(), SecretError> {
    let pat = value.trim();
    if pat.is_empty() {
        return Err(SecretError::InvalidBoardPat);
    }
    let scope = new_cache_scope()?;
    update_vault(|vault| {
        vault.set_board_pat(Some(pat.to_string()));
        vault.board.as_mut().expect("Board PAT was set").cache_scope = Some(scope);
    })
}

pub fn delete_board_pat() -> Result<(), SecretError> {
    update_vault(|vault| vault.set_board_pat(None))
}

pub fn revoke_board_credentials(expected_scope: &str) -> Result<bool, SecretError> {
    update_board_metadata(expected_scope, |board| board.revoked = true)
}

pub fn set_board_diagram_denied(
    expected_scope: &str,
    diagram_id: &str,
    denied: bool,
) -> Result<bool, SecretError> {
    update_board_metadata(expected_scope, |board| {
        if denied {
            board.denied_diagrams.insert(diagram_id.to_string());
        } else {
            board.denied_diagrams.remove(diagram_id);
        }
    })
}

fn update_board_metadata(
    expected_scope: &str,
    edit: impl FnOnce(&mut BoardSecrets),
) -> Result<bool, SecretError> {
    let _guard = io_lock();
    let mut vault = load_raw()?;
    if vault.board_pat().is_none() {
        return Ok(false);
    }
    let board = vault.board.as_mut().expect("Board PAT has a Board secret tree");
    if board.cache_scope.as_deref() != Some(expected_scope) {
        return Ok(false);
    }
    let before = board.clone();
    edit(board);
    if *board != before {
        store_raw(&vault)?;
    }
    Ok(true)
}

#[cfg(test)]
#[path = "../../unit-tests/config/vault/board.rs"]
mod tests;
