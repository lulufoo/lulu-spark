#[cfg(not(test))]
use tauri::webview::{NewWindowResponse, WebviewWindowBuilder};
#[cfg(not(test))]
use tauri::{Url, WindowEvent};
#[cfg(not(test))]
use tauri_plugin_opener::OpenerExt;

#[cfg(not(test))]
fn is_in_app_navigation(url: &Url) -> bool {
    matches!(url.scheme(), "tauri" | "asset" | "file")
}

#[cfg(not(test))]
fn is_localhost_http_url(url: &Url) -> bool {
    if !matches!(url.scheme(), "http" | "https") {
        return false;
    }
    matches!(
        url.host_str(),
        Some("localhost") | Some("127.0.0.1") | Some("::1")
    )
}

#[cfg(not(test))]
fn open_external_url(app: &tauri::AppHandle, url: &Url) {
    if let Err(error) = app.opener().open_url(url.as_str(), None::<&str>) {
        eprintln!("[nav] Failed to open external URL {}: {error}", url);
    }
}

#[cfg(not(test))]
pub(crate) fn create_main_window(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let window_config = app
        .config()
        .app
        .windows
        .iter()
        .find(|w| w.label == "main")
        .or_else(|| app.config().app.windows.first())
        .ok_or("tauri.conf.json must define at least one window")?;

    let app_handle = app.handle().clone();
    let window = WebviewWindowBuilder::from_config(app, window_config)?
        .on_navigation({
            let app_handle = app_handle.clone();
            move |url| {
                if is_in_app_navigation(url) || is_localhost_http_url(url) {
                    return true;
                }
                if url.scheme() == "http" || url.scheme() == "https" {
                    open_external_url(&app_handle, url);
                    return false;
                }
                true
            }
        })
        .on_new_window({
            let app_handle = app_handle.clone();
            move |url, _features| {
                if is_in_app_navigation(&url) || is_localhost_http_url(&url) {
                    return NewWindowResponse::Deny;
                }
                open_external_url(&app_handle, &url);
                NewWindowResponse::Deny
            }
        })
        .build()?;
    eprintln!("[DEBUG-assistant] host: main window created");

    let hide_target = window.clone();
    window.on_window_event(move |event| {
        if let WindowEvent::CloseRequested { api, .. } = event {
            api.prevent_close();
            let _ = hide_target.hide();
        }
    });
    Ok(())
}

pub(crate) fn present_main_window(app: &tauri::AppHandle) {
    use tauri::Manager;
    if let Some(win) = app.get_webview_window("main") {
        let _ = win.unminimize();
        let _ = win.show();
        let _ = win.set_focus();
    }
}

#[cfg(test)]
#[path = "../unit-tests/host/window.rs"]
mod tests;
