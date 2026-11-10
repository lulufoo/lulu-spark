use super::*;

fn url(raw: &str) -> tauri::Url {
    tauri::Url::parse(raw).unwrap_or_else(|_| panic!("parse {raw}"))
}

#[test]
fn the_sketch_sandbox_loads_in_the_preview_frame() {
    assert!(allow_webview_navigation(&url("https://sandbox.luluboard.app/")));
    assert!(allow_webview_navigation(&url(
        "https://sandbox.luluboard.app/index.html"
    )));
    assert!(allow_webview_navigation(&url(
        "https://SANDBOX.luluboard.app/"
    )));
    assert!(allow_webview_navigation(&url(
        "https://sandbox.luluboard.app:443/"
    )));
}

#[test]
fn other_web_pages_still_leave_the_window() {
    assert!(!allow_webview_navigation(&url("https://luluboard.app/")));
    assert!(!allow_webview_navigation(&url("https://example.com/")));
    assert!(!allow_webview_navigation(&url(
        "https://sandbox.luluboard.app.evil.test/"
    )));
    assert!(!allow_webview_navigation(&url(
        "https://user@sandbox.luluboard.app/"
    )));
    assert!(!allow_webview_navigation(&url("http://sandbox.luluboard.app/")));
    assert!(!allow_webview_navigation(&url(
        "https://sandbox.luluboard.app:8443/"
    )));
}

#[test]
fn app_and_localhost_pages_stay_inside() {
    assert!(allow_webview_navigation(&url("tauri://localhost/")));
    assert!(allow_webview_navigation(&url("http://127.0.0.1:5173/")));
    assert!(allow_webview_navigation(&url("https://localhost/app")));
}

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