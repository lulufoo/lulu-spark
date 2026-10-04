fn main() {
    // tauri-codegen embeds icons/icon.icns into the macOS `tauri dev` binary.
    // Without these lines, replacing the icns leaves the old W baked in.
    println!("cargo:rerun-if-changed=icons/icon.icns");
    println!("cargo:rerun-if-changed=icons/icon.png");
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
