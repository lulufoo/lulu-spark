use std::path::PathBuf;

fn lib_rs_source() -> String {
    std::fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src")
            .join("lib.rs"),
    )
    .expect("read src-tauri/src/lib.rs")
}

#[test]
fn lib_starts_discovery_only_after_gateway_has_started() {
    let src = lib_rs_source();
    assert!(
        src.contains("discovery::DiscoveryState")
            || src.contains("services::discovery::DiscoveryState"),
        "lib.rs must own a DiscoveryState tied to Gateway lifecycle"
    );
    assert!(
        src.contains("discovery.try_start"),
        "lib.rs must start discovery from the Gateway-started path"
    );
    let gateway_set = src
        .find("gateway.set")
        .expect("Gateway start must store the handle");
    let discovery_start = src.find("discovery.try_start").expect("discovery start");
    assert!(
        discovery_start > gateway_set,
        "publish _lulu-workbench._tcp only after Gateway has started"
    );
}

#[test]
fn lib_stops_discovery_when_gateway_stops() {
    let src = lib_rs_source();
    assert!(
        src.contains("discovery.stop()"),
        "Gateway stop must withdraw the discovery record"
    );
    let discovery_stop = src.find("discovery.stop()").expect("discovery stop");
    let gateway_stop = src.rfind("gateway.stop()").expect("gateway stop");
    assert!(
        discovery_stop < gateway_stop,
        "withdraw discovery before or as Gateway stops"
    );
}

#[test]
fn lib_does_not_rollback_gateway_when_discovery_fails() {
    let src = lib_rs_source();
    assert!(
        src.contains("discovery.try_start"),
        "discovery start must be best-effort"
    );
    assert_eq!(
        src.matches("gateway.stop()").count(),
        1,
        "discovery failure must not stop or roll back Gateway"
    );
}
