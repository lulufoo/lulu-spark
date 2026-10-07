use super::*;

#[test]
fn present_main_window_shows_and_focuses_the_labeled_main_window() {
    let _present: fn(&tauri::AppHandle) = present_main_window;
    let prod = include_str!("../../host/window.rs");
    assert!(
        prod.contains("#[path = \"../unit-tests/host/window.rs\"]"),
        "production file only declares that tests exist"
    );
    assert!(!prod.contains("#[test]"), "tests belong in the test tree");
    assert!(prod.contains("fn present_main_window"));
    assert!(prod.contains("get_webview_window(\"main\")"));
    assert!(prod.contains("unminimize"));
    assert!(prod.contains(".show()"));
    assert!(prod.contains("set_focus"));
    assert!(
        !prod.contains("Server::http") && !prod.contains("TcpListener"),
        "window presentation must not bind HTTP"
    );
}