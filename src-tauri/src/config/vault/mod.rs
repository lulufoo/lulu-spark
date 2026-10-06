//! One secret tree for LLM, MCP slot tickets, and the bind device ledger.
//!
//! Release: Keychain `lulu-spark` / `vault` (JSON). Debug: the same tree in
//! `dev-secrets.toml`. Tests and `TestSandbox` stay in process memory.

mod codec;
mod store;
mod types;

pub use codec::*;
pub use store::{
    delete_llm_api_key, get_llm_api_key, read_vault, set_llm_api_key, update_vault,
    ACCOUNT_VAULT, KEYCHAIN_SERVICE,
};
pub use types::*;

#[cfg(test)]
pub use store::{
    test_clear_llm, test_clear_scope, test_clear_store_fail, test_fail_store, test_vault_json,
};
