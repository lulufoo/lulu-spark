use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

const SLOT_SPARK: &str = "spark";
const SLOT_CURSOR: &str = "cursor";
const SLOT_CURSOR_IDE: &str = "cursor_ide";
const SLOT_CODEX: &str = "codex";
const SLOT_CLAUDE: &str = "claude";

#[derive(Debug)]
pub enum SecretError {
    Store(String),
    Parse(String),
    InvalidEngine(String),
}

impl std::fmt::Display for SecretError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SecretError::Store(e) => write!(f, "store: {e}"),
            SecretError::Parse(e) => write!(f, "parse: {e}"),
            SecretError::InvalidEngine(e) => write!(f, "invalid llm engine: {e}"),
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
    #[serde(default, skip_serializing_if = "Option::is_none", rename = "supabase-auth")]
    pub supabase_auth: Option<AuthSession>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LlmSecrets {
    /// One credential per LLM engine id (`host`, `openai`, ...).
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub keys: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct McpOauthSecrets {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spark: Option<SlotTicket>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<SlotTicket>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor_ide: Option<SlotTicket>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub codex: Option<SlotTicket>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claude: Option<SlotTicket>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SlotTicket {
    pub handle: String,
    pub state: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub env_suffix: Option<String>,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthSession {
    pub access_token: String,
    pub refresh_token: String,
    pub expires_at: i64,
    pub user: AuthUser,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthUser {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub avatar: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
}

impl Vault {
    pub fn is_empty(&self) -> bool {
        self.has_no_llm_keys()
            && self.mcp_slot(SLOT_SPARK).is_none()
            && self.mcp_slot(SLOT_CURSOR).is_none()
            && self.mcp_slot(SLOT_CODEX).is_none()
            && self.mcp_slot(SLOT_CLAUDE).is_none()
            && self.devices().is_empty()
            && self.auth_session().is_none()
    }

    pub fn llm_api_key(&self, engine: &str) -> Option<&str> {
        self.llm
            .as_ref()
            .and_then(|llm| llm.keys.get(engine))
            .map(String::as_str)
            .filter(|key| !key.is_empty())
    }

    pub fn set_llm_api_key(&mut self, engine: &str, value: Option<String>) {
        match value.filter(|key| !key.is_empty()) {
            Some(key) => {
                self.llm
                    .get_or_insert_with(LlmSecrets::default)
                    .keys
                    .insert(engine.to_string(), key);
            }
            None => {
                if let Some(llm) = self.llm.as_mut() {
                    llm.keys.remove(engine);
                }
            }
        }
        self.prune_llm();
    }

    fn has_no_llm_keys(&self) -> bool {
        self.llm
            .as_ref()
            .map_or(true, |llm| llm.keys.values().all(|key| key.is_empty()))
    }

    fn prune_llm(&mut self) {
        if self.has_no_llm_keys() {
            self.llm = None;
        }
    }

    pub fn mcp_slot(&self, slot: &str) -> Option<&SlotTicket> {
        let mcp = self.mcp_oauth.as_ref()?;
        match slot {
            SLOT_SPARK => mcp.spark.as_ref(),
            SLOT_CURSOR | SLOT_CURSOR_IDE => mcp.cursor.as_ref().or(mcp.cursor_ide.as_ref()),
            SLOT_CODEX => mcp.codex.as_ref(),
            SLOT_CLAUDE => mcp.claude.as_ref(),
            _ => None,
        }
    }

    pub fn set_mcp_slot(&mut self, slot: &str, ticket: Option<SlotTicket>) {
        let mcp = self.mcp_oauth.get_or_insert_with(McpOauthSecrets::default);
        match slot {
            SLOT_SPARK => mcp.spark = ticket,
            SLOT_CURSOR | SLOT_CURSOR_IDE => {
                mcp.cursor = ticket;
                mcp.cursor_ide = None;
            }
            SLOT_CODEX => mcp.codex = ticket,
            SLOT_CLAUDE => mcp.claude = ticket,
            _ => {}
        }
        if mcp_oauth_empty(mcp) {
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

    pub fn auth_session(&self) -> Option<&AuthSession> {
        self.supabase_auth.as_ref()
    }

    pub fn set_auth_session(&mut self, session: Option<AuthSession>) {
        self.supabase_auth = session;
    }

    pub(super) fn prune(&mut self) {
        self.prune_llm();
        if let Some(mcp) = &self.mcp_oauth {
            if mcp_oauth_empty(mcp) {
                self.mcp_oauth = None;
            }
        }
        if self.devices().is_empty() {
            self.bind = None;
        }
        if self.auth_session().is_none() {
            self.supabase_auth = None;
        }
    }
}

fn mcp_oauth_empty(mcp: &McpOauthSecrets) -> bool {
    mcp.spark.is_none()
        && mcp.cursor.is_none()
        && mcp.cursor_ide.is_none()
        && mcp.codex.is_none()
        && mcp.claude.is_none()
}
