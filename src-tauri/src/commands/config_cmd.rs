use serde_json::Value;
use tauri::AppHandle;

use crate::config::secrets;
use crate::config::settings;

#[tauri::command]
pub fn set_config(_app: AppHandle, payload: Value) -> Result<Value, String> {
    if let Err(e) = secrets::apply_token_payload(&payload) {
        return Ok(secrets::secret_error_json(&e));
    }
    let mut settings = settings::load().map_err(|e| format!("{e}"))?;
    settings::apply_config_payload(&mut settings, &payload);
    settings::save(&settings).map_err(|e| format!("{e}"))?;
    Ok(settings::to_config_json(
        &settings,
        secrets::has_github_token(),
        secrets::has_meili_key(),
    ))
}

#[cfg(test)]
mod tests {
    use crate::config::secrets::{self, KEY_GITHUB_TOKEN};
    use crate::config::settings;
    use std::sync::{Mutex, OnceLock};

    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn with_config<F: FnOnce()>(f: F) {
        let _g = ENV_LOCK.get_or_init(|| Mutex::new(())).lock().expect("lock");
        let dir = tempfile::tempdir().expect("tmp");
        settings::set_test_config_dir(Some(dir.path().to_path_buf()));
        secrets::test_secrets_clear();
        f();
        settings::set_test_config_dir(None);
        secrets::test_secrets_clear();
    }

    #[test]
    fn set_config_payload_updates_settings_and_token_flag() {
        with_config(|| {
            let payload = serde_json::json!({
                "archive_root": "/tmp/my-archive",
                "github_token": "secret-pat"
            });
            secrets::apply_token_payload(&payload).expect("secrets");
            let mut s = settings::load().expect("load");
            settings::apply_config_payload(&mut s, &payload);
            settings::save(&s).expect("save");
            let v = settings::to_config_json(&s, secrets::has_github_token(), false);
            assert_eq!(v["archive_root"], "/tmp/my-archive");
            assert_eq!(v["has_github_token"], true);
            assert!(v.get("github_token").is_none());
            let s2 = settings::load().expect("reload");
            assert_eq!(s2.corpus_root.to_string_lossy(), "/tmp/my-archive");
            assert_eq!(
                secrets::get_secret(KEY_GITHUB_TOKEN).expect("get"),
                Some("secret-pat".to_string())
            );
        });
    }
}
