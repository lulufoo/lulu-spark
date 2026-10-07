use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::Mutex;

use serde_json::{Map, Value};

use crate::commands::os_notification::{
    build_clicked_event_payload, map_un_deliver, map_un_flag, show_os_notification_with,
    user_info_with_scheme, validate_os_notification_scheme, NotificationNative,
    OS_NOTIFICATION_CLICKED_EVENT,
};
use crate::test_support::read_rs_dir;

fn source(rel: &str) -> String {
    fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel))
        .unwrap_or_else(|err| panic!("{rel}: {err}"))
}

struct RecordingNative {
    authorized_on_request: bool,
    authorized_on_check: bool,
    request_calls: AtomicU32,
    check_calls: AtomicU32,
    deliver_calls: AtomicU32,
    last_deliver: Mutex<Option<(String, String, Map<String, Value>)>>,
}

impl RecordingNative {
    fn new(authorized_on_request: bool, authorized_on_check: bool) -> Self {
        Self {
            authorized_on_request,
            authorized_on_check,
            request_calls: AtomicU32::new(0),
            check_calls: AtomicU32::new(0),
            deliver_calls: AtomicU32::new(0),
            last_deliver: Mutex::new(None),
        }
    }
}

impl NotificationNative for RecordingNative {
    fn request_authorization(&self) -> Result<bool, String> {
        self.request_calls.fetch_add(1, Ordering::SeqCst);
        Ok(self.authorized_on_request)
    }

    fn read_authorization_status(&self) -> Result<bool, String> {
        self.check_calls.fetch_add(1, Ordering::SeqCst);
        Ok(self.authorized_on_check)
    }

    fn deliver_notification(
        &self,
        title: &str,
        body: &str,
        user_info: &Map<String, Value>,
    ) -> Result<(), String> {
        self.deliver_calls.fetch_add(1, Ordering::SeqCst);
        *self.last_deliver.lock().expect("deliver lock") =
            Some((title.to_string(), body.to_string(), user_info.clone()));
        Ok(())
    }
}

#[test]
fn validate_accepts_spark_scheme() {
    assert!(validate_os_notification_scheme("spark://notes/open?id=a").is_ok());
    assert!(validate_os_notification_scheme("spark://read-later/list").is_ok());
}

#[test]
fn validate_rejects_scheme_without_spark_prefix() {
    for scheme in [
        "",
        "https://example.com",
        "notes/open",
        "Spark://notes/open",
        "spark:/notes/open",
        " spark://notes/open",
    ] {
        assert!(
            validate_os_notification_scheme(scheme).is_err(),
            "expected Err for {scheme:?}"
        );
    }
}

#[test]
fn user_info_writes_scheme_verbatim() {
    let scheme = "spark://notes/open?id=a&path=p.md";
    let info = user_info_with_scheme(scheme);
    assert_eq!(info.get("scheme").and_then(|v| v.as_str()), Some(scheme));
}

#[test]
fn clicked_payload_is_scheme_object() {
    let scheme = "spark://read-later/list";
    assert_eq!(
        build_clicked_event_payload(scheme),
        serde_json::json!({ "scheme": scheme })
    );
    assert_eq!(OS_NOTIFICATION_CLICKED_EVENT, "os-notification:clicked");
}

#[test]
fn invalid_scheme_does_not_request_permission_or_deliver() {
    let probed = AtomicBool::new(false);
    let native = RecordingNative::new(true, true);
    let err = show_os_notification_with(
        &probed,
        &native,
        "New note",
        "A note was added",
        "https://evil.example",
    )
    .expect_err("invalid scheme");
    assert!(!err.is_empty());
    assert_eq!(native.request_calls.load(Ordering::SeqCst), 0);
    assert_eq!(native.check_calls.load(Ordering::SeqCst), 0);
    assert_eq!(native.deliver_calls.load(Ordering::SeqCst), 0);
    assert!(!probed.load(Ordering::SeqCst));
}

#[test]
fn first_authorized_call_requests_permission_and_delivers_user_info() {
    let probed = AtomicBool::new(false);
    let native = RecordingNative::new(true, true);
    let scheme = "spark://notes/open?id=n1&path=raw%2Fa.md";
    show_os_notification_with(&probed, &native, "New note", "A note was added", scheme)
        .expect("authorized first call");
    assert_eq!(native.request_calls.load(Ordering::SeqCst), 1);
    assert_eq!(native.check_calls.load(Ordering::SeqCst), 0);
    assert_eq!(native.deliver_calls.load(Ordering::SeqCst), 1);
    assert!(probed.load(Ordering::SeqCst));
    let delivered = native
        .last_deliver
        .lock()
        .expect("deliver lock")
        .clone()
        .expect("delivered");
    assert_eq!(delivered.0, "New note");
    assert_eq!(delivered.1, "A note was added");
    assert_eq!(
        delivered.2.get("scheme").and_then(|v| v.as_str()),
        Some(scheme)
    );
}

#[test]
fn authorized_send_accepts_notes_pass_scheme() {
    let probed = AtomicBool::new(true);
    let native = RecordingNative::new(true, true);
    let id = "trace_12345678abcd";
    let bag = urlencoding::encode(&format!(r#"{{"id":"{id}"}}"#)).into_owned();
    let scheme = format!("spark://notes/open?id=n1&path=raw%2Fa.md&pass={bag}");
    show_os_notification_with(&probed, &native, "New note", "A note was added", &scheme)
        .expect("authorized pass scheme");
    assert_eq!(native.deliver_calls.load(Ordering::SeqCst), 1);
    let delivered = native
        .last_deliver
        .lock()
        .expect("deliver lock")
        .clone()
        .expect("delivered");
    assert_eq!(
        delivered.2.get("scheme").and_then(|v| v.as_str()),
        Some(scheme.as_str())
    );
}

#[test]
fn subsequent_call_only_reads_authorization_status() {
    let probed = AtomicBool::new(true);
    let native = RecordingNative::new(true, true);
    show_os_notification_with(
        &probed,
        &native,
        "Read Later",
        "A link was saved",
        "spark://read-later/list",
    )
    .expect("authorized subsequent call");
    assert_eq!(native.request_calls.load(Ordering::SeqCst), 0);
    assert_eq!(native.check_calls.load(Ordering::SeqCst), 1);
    assert_eq!(native.deliver_calls.load(Ordering::SeqCst), 1);
}

#[test]
fn unauthorized_first_call_does_not_deliver() {
    let probed = AtomicBool::new(false);
    let native = RecordingNative::new(false, true);
    assert!(show_os_notification_with(
        &probed,
        &native,
        "New note",
        "A note was added",
        "spark://notes/open?id=a",
    )
    .is_err());
    assert_eq!(native.request_calls.load(Ordering::SeqCst), 1);
    assert_eq!(native.deliver_calls.load(Ordering::SeqCst), 0);
    assert!(probed.load(Ordering::SeqCst));
}

#[test]
fn unauthorized_subsequent_call_does_not_deliver() {
    let probed = AtomicBool::new(true);
    let native = RecordingNative::new(true, false);
    assert!(show_os_notification_with(
        &probed,
        &native,
        "New note",
        "A note was added",
        "spark://notes/open?id=a",
    )
    .is_err());
    assert_eq!(native.request_calls.load(Ordering::SeqCst), 0);
    assert_eq!(native.check_calls.load(Ordering::SeqCst), 1);
    assert_eq!(native.deliver_calls.load(Ordering::SeqCst), 0);
}

#[test]
fn map_un_codes_distinguish_no_bundle() {
    assert_eq!(map_un_flag(1, "request"), Ok(true));
    assert_eq!(map_un_flag(0, "request"), Ok(false));
    assert!(map_un_flag(-2, "request")
        .unwrap_err()
        .contains("CFBundleIdentifier"));
    assert_eq!(
        map_un_flag(-1, "request").unwrap_err(),
        "UserNotifications request failed"
    );
    assert!(map_un_deliver(1).is_ok());
    assert!(map_un_deliver(-2)
        .unwrap_err()
        .contains("CFBundleIdentifier"));
    assert_eq!(
        map_un_deliver(-1).unwrap_err(),
        "UserNotifications deliver failed"
    );
}

#[test]
fn native_objc_guards_un_when_bundle_identifier_missing() {
    let objc = source("native/os_notification.m");
    assert!(objc.contains("return bid.length > 0 ? 0 : -2"));
    assert_eq!(objc.matches("currentNotificationCenter").count(), 4);
    assert!(objc.matches("spark_un_unavailable()").count() >= 4);
}

#[cfg(target_os = "macos")]
#[test]
fn unpackaged_test_binary_skips_un_without_abort() {
    use std::os::raw::c_char;
    extern "C" {
        fn spark_un_request_authorization() -> i32;
        fn spark_un_read_authorization() -> i32;
        fn spark_un_set_click_callback(cb: extern "C" fn(*const c_char));
    }
    extern "C" fn noop(_: *const c_char) {}
    unsafe { spark_un_set_click_callback(noop) };
    assert_eq!(unsafe { spark_un_request_authorization() }, -2);
    assert_eq!(unsafe { spark_un_read_authorization() }, -2);
}

#[test]
fn command_is_thin_native_and_registered() {
    let cmd = source("src/commands/os_notification.rs");
    let lib = source("src/lib.rs");
    assert!(
        cmd.contains("fn show_os_notification"),
        "command fn show_os_notification must exist"
    );
    assert!(
        cmd.contains("title: String")
            && cmd.contains("body: String")
            && cmd.contains("scheme: String"),
        "thin command must only take title, body, scheme"
    );
    assert!(
        !cmd.contains("message_center") && !cmd.contains("Envelope"),
        "thin command must not interpret envelopes or write message_center"
    );
    assert!(
        !cmd.contains("tauri_plugin_notification") && !cmd.contains("tauri-plugin-notification"),
        "must not use tauri-plugin-notification"
    );
    assert!(
        cmd.contains("UserNotifications")
            || cmd.contains("user_notifications")
            || cmd.contains("didReceiveNotificationResponse"),
        "must go through native UserNotifications (or equivalent)"
    );
    assert!(
        cmd.contains("userInfo") || cmd.contains("user_info"),
        "scheme must be written to UN userInfo"
    );
    assert!(
        cmd.contains("emit_os_notification_clicked") && cmd.contains("os-notification:clicked"),
        "click path must emit os-notification:clicked"
    );
    assert!(
        cmd.contains("NODE_NOTIFY_SEND") && cmd.contains("log_hop"),
        "deliver path must log notify.send"
    );
    assert!(
        cmd.contains("pass_id_from_scheme") && cmd.contains("business_from_scheme_host"),
        "deliver hop must take id from pass and business from host map"
    );
    assert!(
        !cmd.contains("trace_from_scheme"),
        "must not read hop id from trace query"
    );
    assert!(
        cmd.contains("log_click_native") && !cmd.contains("fn log_os_notify_hop"),
        "click hops go through app_log; do not keep a notify-only command"
    );
    assert!(
        lib.contains("commands::os_notification::show_os_notification"),
        "generate_handler must register show_os_notification"
    );
    assert!(
        lib.contains("commands::app_log::log_app_event"),
        "generate_handler must register log_app_event"
    );
    assert!(
        lib.contains("RunEvent::Opened") && lib.contains("log_scheme_open"),
        "scheme-wake via Launch Services must log scheme.open"
    );
}

#[test]
fn production_module_declares_tests_without_test_bodies() {
    let cmd = source("src/commands/os_notification.rs");
    assert!(
        cmd.contains("#[path = \"../unit-tests/commands/os_notification.rs\"]"),
        "os_notification.rs must declare the unit-tests path"
    );
    let commands_dir = read_rs_dir(concat!(env!("CARGO_MANIFEST_DIR"), "/src/commands"));
    assert!(
        !commands_dir.contains("#[test]"),
        "test bodies must not live in production command files"
    );
}
