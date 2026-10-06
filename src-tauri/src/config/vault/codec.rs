use super::types::{SecretError, Vault};

pub fn vault_to_json(vault: &Vault) -> Result<String, SecretError> {
    serde_json::to_string(vault).map_err(|e| SecretError::Keyring(e.to_string()))
}

pub fn vault_from_json(raw: &str) -> Result<Vault, SecretError> {
    if raw.trim().is_empty() {
        return Ok(Vault::default());
    }
    serde_json::from_str(raw).map_err(|e| SecretError::Keyring(e.to_string()))
}

pub fn vault_to_toml(vault: &Vault) -> Result<String, SecretError> {
    if vault.is_empty() {
        return Ok(String::new());
    }
    toml::to_string(vault).map_err(|e| SecretError::Keyring(e.to_string()))
}

pub fn vault_from_toml(raw: &str) -> Result<Vault, SecretError> {
    if raw.trim().is_empty() {
        return Ok(Vault::default());
    }
    toml::from_str(raw).map_err(|e| SecretError::Keyring(e.to_string()))
}

#[cfg(test)]
#[path = "../../unit-tests/config/vault/codec.rs"]
mod tests;
