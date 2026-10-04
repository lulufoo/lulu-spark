//! L1/L3 thin command: native UserNotifications, scheme in userInfo,
//! clicks via didReceiveNotificationResponse.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use serde_json::{json, Map, Value};
use tauri::{AppHandle, Emitter};

use crate::services::os_notify_trace::{
    log_click_native, log_hop, trace_from_scheme, NODE_NOTIFY_SEND,
};

pub const OS_NOTIFICATION_CLICKED_EVENT: &str = "os-notification:clicked";
const SCHEME_PREFIX: &str = "spark://";
const UNAUTHORIZED: &str = "notification not authorized";
const UN_NO_BUNDLE: i32 = -2;
const UN_UNAVAILABLE: &str =
    "UserNotifications unavailable: process has no CFBundleIdentifier (unpackaged/dev binary)";

static PERMISSION_PROBED: AtomicBool = AtomicBool::new(false);
static CLICK_APP: Mutex<Option<AppHandle>> = Mutex::new(None);

pub trait NotificationNative {
    fn request_authorization(&self) -> Result<bool, String>;
    fn read_authorization_status(&self) -> Result<bool, String>;
    fn deliver_notification(
        &self,
        title: &str,
        body: &str,
        user_info: &Map<String, Value>,
    ) -> Result<(), String>;
}

pub fn validate_os_notification_scheme(scheme: &str) -> Result<(), String> {
    if scheme.starts_with(SCHEME_PREFIX) {
        Ok(())
    } else {
        Err("scheme must start with spark://".into())
    }
}

pub fn user_info_with_scheme(scheme: &str) -> Map<String, Value> {
    let mut map = Map::new();
    map.insert("scheme".into(), Value::String(scheme.to_string()));
    map
}

pub fn build_clicked_event_payload(scheme: &str) -> Value {
    json!({ "scheme": scheme })
}

pub fn emit_os_notification_clicked(app: &AppHandle, scheme: &str) {
    log_click_native(scheme, true);
    let _ = app.emit(
        OS_NOTIFICATION_CLICKED_EVENT,
        build_clicked_event_payload(scheme),
    );
}

pub fn show_os_notification_with(
    probed: &AtomicBool,
    native: &dyn NotificationNative,
    title: &str,
    body: &str,
    scheme: &str,
) -> Result<(), String> {
    validate_os_notification_scheme(scheme)?;
    let trace = trace_from_scheme(scheme).unwrap_or("trace_missing");
    let first = !probed.swap(true, Ordering::SeqCst);
    let authorized = if first {
        native.request_authorization()?
    } else {
        native.read_authorization_status()?
    };
    if !authorized {
        log_hop(NODE_NOTIFY_SEND, trace, "unauthorized");
        return Err(UNAUTHORIZED.into());
    }
    match native.deliver_notification(title, body, &user_info_with_scheme(scheme)) {
        Ok(()) => {
            log_hop(NODE_NOTIFY_SEND, trace, "ok");
            Ok(())
        }
        Err(err) => {
            log_hop(NODE_NOTIFY_SEND, trace, "deliver_fail");
            Err(err)
        }
    }
}

pub fn map_un_flag(code: i32, action: &str) -> Result<bool, String> {
    match code {
        1 => Ok(true),
        0 => Ok(false),
        UN_NO_BUNDLE => Err(UN_UNAVAILABLE.into()),
        _ => Err(format!("UserNotifications {action} failed")),
    }
}

pub fn map_un_deliver(code: i32) -> Result<(), String> {
    match code {
        1 => Ok(()),
        UN_NO_BUNDLE => Err(UN_UNAVAILABLE.into()),
        _ => Err("UserNotifications deliver failed".into()),
    }
}

fn bind_click_app(app: AppHandle) {
    if let Ok(mut slot) = CLICK_APP.lock() {
        *slot = Some(app);
    }
    macos::install_click_callback();
}

#[tauri::command]
pub fn show_os_notification(
    app: AppHandle,
    title: String,
    body: String,
    scheme: String,
) -> Result<(), String> {
    validate_os_notification_scheme(&scheme)?;
    bind_click_app(app);
    show_os_notification_with(
        &PERMISSION_PROBED,
        &macos::MacOsNotificationNative,
        &title,
        &body,
        &scheme,
    )
}

#[cfg(target_os = "macos")]
mod macos {
    use super::{emit_os_notification_clicked, NotificationNative, CLICK_APP};
    use serde_json::{Map, Value};
    use std::ffi::{CStr, CString};
    use std::os::raw::c_char;

    extern "C" {
        fn spark_un_request_authorization() -> i32;
        fn spark_un_read_authorization() -> i32;
        fn spark_un_deliver(
            title: *const c_char,
            body: *const c_char,
            scheme: *const c_char,
        ) -> i32;
        fn spark_un_set_click_callback(cb: extern "C" fn(*const c_char));
    }

    extern "C" fn on_native_click(scheme: *const c_char) {
        if scheme.is_null() {
            crate::services::os_notify_trace::log_click_native("", false);
            return;
        }
        let scheme = unsafe { CStr::from_ptr(scheme) }
            .to_string_lossy()
            .into_owned();
        let app = CLICK_APP.lock().ok().and_then(|slot| slot.clone());
        if let Some(app) = app {
            emit_os_notification_clicked(&app, &scheme);
        } else {
            crate::services::os_notify_trace::log_click_native(&scheme, false);
        }
    }

    pub struct MacOsNotificationNative;

    impl NotificationNative for MacOsNotificationNative {
        fn request_authorization(&self) -> Result<bool, String> {
            super::map_un_flag(unsafe { spark_un_request_authorization() }, "request")
        }

        fn read_authorization_status(&self) -> Result<bool, String> {
            super::map_un_flag(unsafe { spark_un_read_authorization() }, "status")
        }

        fn deliver_notification(
            &self,
            title: &str,
            body: &str,
            user_info: &Map<String, Value>,
        ) -> Result<(), String> {
            let scheme = user_info
                .get("scheme")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let title = CString::new(title).map_err(|e| e.to_string())?;
            let body = CString::new(body).map_err(|e| e.to_string())?;
            let scheme = CString::new(scheme).map_err(|e| e.to_string())?;
            super::map_un_deliver(unsafe {
                spark_un_deliver(title.as_ptr(), body.as_ptr(), scheme.as_ptr())
            })
        }
    }

    pub fn install_click_callback() {
        unsafe { spark_un_set_click_callback(on_native_click) };
    }
}

#[cfg(not(target_os = "macos"))]
mod macos {
    use super::NotificationNative;
    use serde_json::{Map, Value};

    pub struct MacOsNotificationNative;

    impl NotificationNative for MacOsNotificationNative {
        fn request_authorization(&self) -> Result<bool, String> {
            Err("os notifications require macOS".into())
        }

        fn read_authorization_status(&self) -> Result<bool, String> {
            Err("os notifications require macOS".into())
        }

        fn deliver_notification(
            &self,
            _title: &str,
            _body: &str,
            _user_info: &Map<String, Value>,
        ) -> Result<(), String> {
            Err("os notifications require macOS".into())
        }
    }

    pub fn install_click_callback() {}
}

#[cfg(test)]
#[path = "../unit-tests/commands/os_notification.rs"]
mod os_notification_tests;
