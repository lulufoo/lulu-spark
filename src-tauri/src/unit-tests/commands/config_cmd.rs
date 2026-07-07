use crate::config::secrets::{self, KEY_GITHUB_TOKEN};
use crate::config::settings;
use crate::test_support::TestSandbox;

fn with_config<F: FnOnce()>(f: F) {
    let _sandbox = TestSandbox::new();
    secrets::test_secrets_clear();
    f();
}

#[test]
fn set_config_payload_updates_settings_and_token_flag() {
    with_config(|| {
        let payload = serde_json::json!({
            "workbench_knowledge_root": "/tmp/my-workbench-knowledge",
            "github_token": "secret-pat"
        });
        secrets::apply_token_payload(&payload).expect("secrets");
        let mut s = settings::load().expect("load");
        settings::apply_config_payload(&mut s, &payload);
        settings::save(&s).expect("save");
        let v = settings::to_config_json(&s, secrets::has_github_token(), false);
        assert_eq!(v["workbench_knowledge_root"], "/tmp/my-workbench-knowledge");
        assert_eq!(v["has_github_token"], true);
        assert!(v.get("github_token").is_none());
        let s2 = settings::load().expect("reload");
        assert_eq!(
            s2.workbench_knowledge_root.to_string_lossy(),
            "/tmp/my-workbench-knowledge"
        );
        assert_eq!(
            secrets::get_secret(KEY_GITHUB_TOKEN).expect("get"),
            Some("secret-pat".to_string())
        );
    });
}
