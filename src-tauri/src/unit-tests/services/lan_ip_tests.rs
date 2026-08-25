use std::net::{Ipv4Addr, Ipv6Addr};
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};

use super::*;

fn nic(name: &str, addr: &str, up: bool) -> NicIpv4 {
    NicIpv4 {
        name: name.to_string(),
        addr: addr.parse().expect("ipv4"),
        up,
    }
}

fn with_nics(nics: Vec<NicIpv4>, test: impl FnOnce()) {
    test_override_nics(Some(nics));
    let result = catch_unwind(AssertUnwindSafe(test));
    test_override_nics(None);
    if let Err(payload) = result {
        resume_unwind(payload);
    }
}

#[test]
fn current_lan_ipv4_prefers_en0_rfc1918() {
    with_nics(
        vec![
            nic("en1", "192.168.0.8", true),
            nic("en0", "10.0.0.4", true),
            nic("en0", "1.2.3.4", true),
        ],
        || {
            assert_eq!(
                current_lan_ipv4(),
                Some(Ipv4Addr::new(10, 0, 0, 4)),
                "en0 RFC1918 wins over earlier NICs and over en0 public"
            );
        },
    );
}

#[test]
fn current_lan_ipv4_uses_os_order_when_en0_has_no_rfc1918() {
    with_nics(
        vec![
            nic("en1", "192.168.1.20", true),
            nic("en0", "1.2.3.4", true),
        ],
        || {
            assert_eq!(
                current_lan_ipv4(),
                Some(Ipv4Addr::new(192, 168, 1, 20)),
                "no extra sort: first qualified address in enumeration order"
            );
        },
    );
}

#[test]
fn current_lan_ipv4_skips_down_loopback_and_link_local() {
    with_nics(
        vec![
            nic("lo0", "127.0.0.1", true),
            nic("en0", "169.254.10.2", true),
            nic("en2", "192.168.9.9", false),
            nic("en1", "172.16.1.8", true),
        ],
        || {
            assert_eq!(current_lan_ipv4(), Some(Ipv4Addr::new(172, 16, 1, 8)));
        },
    );
}

#[test]
fn current_lan_ipv4_returns_none_when_no_qualified_address() {
    with_nics(
        vec![
            nic("lo0", "127.0.0.1", true),
            nic("en0", "169.254.1.1", true),
            nic("en1", "10.0.0.2", false),
        ],
        || {
            assert_eq!(current_lan_ipv4(), None);
        },
    );
}

#[test]
fn current_lan_ipv4_rereads_nics_each_call() {
    with_nics(vec![nic("en0", "10.1.1.1", true)], || {
        assert_eq!(current_lan_ipv4(), Some(Ipv4Addr::new(10, 1, 1, 1)));
    });
    with_nics(vec![nic("en0", "10.2.2.2", true)], || {
        assert_eq!(
            current_lan_ipv4(),
            Some(Ipv4Addr::new(10, 2, 2, 2)),
            "must reread; no cached previous address"
        );
    });
}

#[test]
fn current_lan_ipv4_never_returns_loopback_or_link_local_from_live_nics() {
    if let Some(addr) = current_lan_ipv4() {
        assert!(!addr.is_loopback(), "live pick must skip loopback");
        let oct = addr.octets();
        assert!(
            !(oct[0] == 169 && oct[1] == 254),
            "live pick must skip 169.254/16"
        );
    }
}

#[test]
fn crate_exports_lan_ip_and_stays_ipv4_only() {
    let _: fn() -> Option<Ipv4Addr> = current_lan_ipv4;
    let _: fn(&[NicIpv4]) -> Option<Ipv4Addr> = select_lan_ipv4;
    let _ipv6_unused: Option<Ipv6Addr> = None;
    let src = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/services/lan_ip/mod.rs"
    ));
    assert!(
        !src.contains("Host HTTP") && !src.contains("0.0.0.0"),
        "lan_ip must not bind Host HTTP or MCP to the chosen address"
    );
}

#[test]
fn services_mod_registers_lan_ip() {
    let src = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/services/mod.rs"
    ));
    assert!(
        src.contains("pub mod lan_ip"),
        "services/mod.rs must register lan_ip"
    );
}
