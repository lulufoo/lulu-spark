use std::path::PathBuf;

fn build_rs_source() -> String {
    std::fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("build.rs"))
        .expect("read src-tauri/build.rs")
}

#[test]
fn build_rs_rebuilds_when_macos_dev_icon_changes() {
    let src = build_rs_source();
    assert!(
        src.contains("cargo:rerun-if-changed=icons/icon.icns"),
        "macOS tauri dev bakes icon.icns into the binary"
    );
    assert!(
        src.contains("cargo:rerun-if-changed=icons/icon.png"),
        "PNG fallback must also invalidate the baked icon"
    );
}
