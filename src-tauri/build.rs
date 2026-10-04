fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("macos") {
        println!("cargo:rerun-if-changed=native/os_notification.m");
        cc::Build::new()
            .file("native/os_notification.m")
            .flag("-fobjc-arc")
            .compile("spark_os_notification");
        println!("cargo:rustc-link-lib=framework=Foundation");
        println!("cargo:rustc-link-lib=framework=UserNotifications");
    }
    tauri_build::build();
}
