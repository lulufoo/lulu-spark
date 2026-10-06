//! One secret tree for LLM, MCP slot tickets, bind devices, and auth session.
//!
//! Release: Keychain `lulu-spark` / `vault` (JSON). Debug: the same tree in
//! `dev-secrets.toml`. Tests and `TestSandbox` stay in process memory.

mod codec;
mod store;
mod types;

pub use codec::*;
pub use store::{
    delete_auth_session, delete_llm_api_key, get_auth_session, get_llm_api_key, read_vault,
    set_auth_session, set_llm_api_key, update_vault, ACCOUNT_VAULT, KEYCHAIN_SERVICE,
};
pub use types::*;

#[cfg(test)]
pub use store::{
    test_clear_llm, test_clear_scope, test_clear_store_fail, test_fail_store, test_vault_json,
};
