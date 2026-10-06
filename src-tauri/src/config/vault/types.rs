use serde::{Deserialize, Serialize};

pub const KEY_LLM_API_KEY: &str = "llm_api_key";

const SLOT_SPARK: &str = "spark";
const SLOT_CURSOR_IDE: &str = "cursor_ide";

#[derive(Debug)]
pub enum SecretError {
    Keyring(String),
    Poisoned,
}

impl std::fmt::Display for SecretError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecretError::Keyring(e) => write!(f, "keyring: {e}"),
            SecretError::Poisoned => write!(f, "lock poisoned"),
        }
    }
}

impl std::error::Error for SecretError {}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Vault {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub llm: Option<LlmSecrets>,
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "mcp-oauth")]
    pub mcp_oauth: Option<McpOauthSecrets>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub bind: Option<BindSecrets>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LlmSecrets {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub llm_api_key: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpOauthSecrets {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spark: Option<SlotTicket>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor_ide: Option<SlotTicket>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotTicket {
    pub handle: String,
    pub state: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BindSecrets {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub devices: Vec<BindDevice>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BindDevice {
    pub device_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_label: Option<String>,
    pub token_hash: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token_hint: Option<String>,
    pub revoked: bool,
}

impl Vault {
    pub fn is_empty(&self) -> bool {
        self.llm_api_key().is_none()
            && self.mcp_slot(SLOT_SPARK).is_none()
            && self.mcp_slot(SLOT_CURSOR_IDE).is_none()
            && self.devices().is_empty()
    }

    pub fn llm_api_key(&self) -> Option<&str> {
        self.llm
            .as_ref()
            .and_then(|llm| llm.llm_api_key.as_deref())
            .filter(|key| !key.is_empty())
    }

    pub fn set_llm_api_key(&mut self, value: Option<String>) {
        match value.filter(|key| !key.is_empty()) {
            Some(key) => {
                self.llm = Some(LlmSecrets {
                    llm_api_key: Some(key),
                });
            }
            None => self.llm = None,
        }
    }

    pub fn mcp_slot(&self, slot: &str) -> Option<&SlotTicket> {
        let mcp = self.mcp_oauth.as_ref()?;
        match slot {
            SLOT_SPARK => mcp.spark.as_ref(),
            SLOT_CURSOR_IDE => mcp.cursor_ide.as_ref(),
            _ => None,
        }
    }

    pub fn set_mcp_slot(&mut self, slot: &str, ticket: Option<SlotTicket>) {
        let mcp = self.mcp_oauth.get_or_insert_with(McpOauthSecrets::default);
        match slot {
            SLOT_SPARK => mcp.spark = ticket,
            SLOT_CURSOR_IDE => mcp.cursor_ide = ticket,
            _ => {}
        }
        if mcp.spark.is_none() && mcp.cursor_ide.is_none() {
            self.mcp_oauth = None;
        }
    }

    pub fn devices(&self) -> &[BindDevice] {
        self.bind
            .as_ref()
            .map(|bind| bind.devices.as_slice())
            .unwrap_or(&[])
    }

    pub fn set_devices(&mut self, devices: Vec<BindDevice>) {
        if devices.is_empty() {
            self.bind = None;
        } else {
            self.bind = Some(BindSecrets { devices });
        }
    }

    pub(super) fn prune(&mut self) {
        if self.llm_api_key().is_none() {
            self.llm = None;
        }
        if let Some(mcp) = &self.mcp_oauth {
            if mcp.spark.is_none() && mcp.cursor_ide.is_none() {
                self.mcp_oauth = None;
            }
        }
        if self.devices().is_empty() {
            self.bind = None;
        }
    }
}
