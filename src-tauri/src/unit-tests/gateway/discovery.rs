use std::net::Ipv4Addr;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::*;
use crate::config::settings::{DEFAULT_PROD_GATEWAY_PORT, DEFAULT_SANDBOX_GATEWAY_PORT};
use crate::host::lan_ip::{test_override_nics, NicIpv4};

fn nic(name: &str, addr: &str) -> NicIpv4 {
    NicIpv4 {
        name: name.to_string(),
        addr: addr.parse().expect("ipv4"),
        up: true,
    }
}

fn with_nics(nics: Option<Vec<NicIpv4>>, test: impl FnOnce()) {
    test_override_nics(nics);
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(test));
    test_override_nics(None);
    if let Err(payload) = result {
        std::panic::resume_unwind(payload);
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum MockEvent {
    Publish(AdvertisedRecord),
    Withdraw,
}

#[derive(Clone, Default)]
struct MockPublisher {
    events: Arc<Mutex<Vec<MockEvent>>>,
}

impl MockPublisher {
    fn events(&self) -> Vec<MockEvent> {
        self.events.lock().expect("events").clone()
    }

    fn published(&self) -> Vec<AdvertisedRecord> {
        self.events()
            .into_iter()
            .filter_map(|event| match event {
                MockEvent::Publish(record) => Some(record),
                MockEvent::Withdraw => None,
            })
            .collect()
    }
}

impl Publisher for MockPublisher {
    fn publish(&mut self, record: AdvertisedRecord) -> Result<(), String> {
        self.events
            .lock()
            .expect("events")
            .push(MockEvent::Publish(record));
        Ok(())
    }

    fn withdraw(&mut self) {
        self.events
            .lock()
            .expect("events")
            .push(MockEvent::Withdraw);
    }
}

fn start_mock(port: u16) -> (DiscoveryHandle, MockPublisher) {
    let publisher = MockPublisher::default();
    let handle = start_with(DiscoveryConfig::for_gateway_port(port), publisher.clone());
    (handle, publisher)
}

#[test]
fn gateway_start_publishes_lulu_workbench_tcp_with_lan_ip_and_port() {
    with_nics(Some(vec![nic("en0", "192.168.1.8")]), || {
        let (handle, publisher) = start_mock(DEFAULT_PROD_GATEWAY_PORT);
        let published = publisher.published();
        assert_eq!(published.len(), 1, "Gateway start is what publishes discovery");
        assert_eq!(published[0].service_type, SERVICE_TYPE);
        assert_eq!(SERVICE_TYPE, "_lulu-workbench._tcp");
        assert_eq!(published[0].ipv4, Ipv4Addr::new(192, 168, 1, 8));
        assert_eq!(published[0].port, 7654);
        assert_eq!(
            current_advertisement(&handle),
            Some(published[0].clone())
        );
        stop(handle);
    });
}

#[test]
fn advertisement_uses_sandbox_gateway_port() {
    with_nics(Some(vec![nic("en0", "10.0.0.4")]), || {
        let (handle, publisher) = start_mock(DEFAULT_SANDBOX_GATEWAY_PORT);
        let published = publisher.published();
        assert_eq!(published.len(), 1);
        assert_eq!(published[0].ipv4, Ipv4Addr::new(10, 0, 0, 4));
        assert_eq!(published[0].port, 17654);
        stop(handle);
    });
}

#[test]
fn stop_withdraws_the_published_record() {
    with_nics(Some(vec![nic("en0", "192.168.1.8")]), || {
        let (handle, publisher) = start_mock(DEFAULT_PROD_GATEWAY_PORT);
        stop(handle);
        assert_eq!(
            publisher.events(),
            vec![
                MockEvent::Publish(AdvertisedRecord {
                    service_type: SERVICE_TYPE.to_string(),
                    ipv4: Ipv4Addr::new(192, 168, 1, 8),
                    port: 7654,
                }),
                MockEvent::Withdraw,
            ]
        );
    });
}

#[test]
fn publish_and_refresh_both_reread_current_lan_ipv4() {
    with_nics(Some(vec![nic("en0", "10.1.1.1")]), || {
        let (handle, publisher) = start_mock(DEFAULT_PROD_GATEWAY_PORT);
        assert_eq!(
            publisher.published().last().map(|r| r.ipv4),
            Some(Ipv4Addr::new(10, 1, 1, 1)),
            "first publish must live-read current_lan_ipv4()"
        );
        test_override_nics(Some(vec![nic("en0", "10.2.2.2")]));
        refresh(&handle);
        let published = publisher.published();
        assert_eq!(published.len(), 2, "refresh must publish again from a live read");
        assert_eq!(published[1].ipv4, Ipv4Addr::new(10, 2, 2, 2));
        assert_eq!(published[1].port, 7654);
        assert_eq!(published[1].service_type, SERVICE_TYPE);
        stop(handle);
    });
}

#[test]
fn default_refresh_interval_is_thirty_seconds() {
    assert_eq!(DEFAULT_REFRESH_INTERVAL, Duration::from_secs(30));
    let config = DiscoveryConfig::for_gateway_port(7654);
    assert_eq!(config.gateway_port, 7654);
    assert_eq!(config.refresh_interval, Duration::from_secs(30));
}

#[test]
fn missing_lan_ip_does_not_advertise() {
    with_nics(Some(vec![nic("lo0", "127.0.0.1")]), || {
        let (handle, publisher) = start_mock(DEFAULT_PROD_GATEWAY_PORT);
        assert!(
            publisher.events().is_empty(),
            "no qualified LAN IP must not publish"
        );
        assert_eq!(current_advertisement(&handle), None);
        stop(handle);
        assert!(
            publisher.events().is_empty(),
            "stop must not withdraw a record that was never published"
        );
    });
}

#[test]
fn lost_lan_ip_withdraws_existing_record() {
    with_nics(Some(vec![nic("en0", "172.16.1.8")]), || {
        let (handle, publisher) = start_mock(DEFAULT_SANDBOX_GATEWAY_PORT);
        assert_eq!(publisher.published().len(), 1);
        test_override_nics(Some(vec![nic("lo0", "127.0.0.1")]));
        refresh(&handle);
        assert_eq!(
            publisher.events().last(),
            Some(&MockEvent::Withdraw),
            "refresh with no LAN IP must withdraw"
        );
        assert_eq!(current_advertisement(&handle), None);
        stop(handle);
    });
}

#[test]
fn advertisement_does_not_carry_health_credentials_or_tls_identity() {
    with_nics(Some(vec![nic("en0", "192.168.9.9")]), || {
        let (handle, publisher) = start_mock(DEFAULT_PROD_GATEWAY_PORT);
        let record = publisher.published().pop().expect("published");
        assert!(
            record.txt_pairs().is_empty(),
            "discovery must not advertise extra TXT identity material"
        );
        let debug = format!("{record:?}");
        for needle in [
            "health",
            "Health",
            "token",
            "credential",
            "tls",
            "fingerprint",
            "password",
        ] {
            assert!(
                !debug.contains(needle),
                "advertisement must not prove identity via {needle}"
            );
        }
        stop(handle);
    });
}

#[test]
fn discovery_uses_system_mdns_not_homemade_udp_or_network_listener() {
    let src = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gateway/discovery.rs"
    ));
    let cargo = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/Cargo.toml"));
    assert!(
        src.contains("dns_sd") || src.contains("DNSService"),
        "must use system mDNS/Bonjour (dns_sd / DNSServiceRegister)"
    );
    assert!(
        !src.contains("mdns_sd") && !cargo.contains("mdns-sd"),
        "must not use a homemade mDNS stack"
    );
    for needle in ["5353", "UdpSocket", "Ipv4Addr::new(224, 0, 0, 251)"] {
        assert!(
            !src.contains(needle),
            "must not roll a UDP mDNS/broadcast path ({needle})"
        );
    }
    for needle in [
        "SCNetworkReachability",
        "NWPathMonitor",
        "SystemConfiguration",
        "network_change",
        "notify::recommended_watcher",
    ] {
        assert!(
            !src.contains(needle),
            "must not attach an OS network-change listener ({needle})"
        );
    }
}

#[test]
fn gateway_registers_discovery() {
    let gateway = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gateway/mod.rs"
    ));
    let services = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/services/mod.rs"
    ));
    assert!(
        gateway.contains("pub mod discovery"),
        "gateway must own discovery"
    );
    assert!(
        !services.contains("pub mod discovery"),
        "discovery must not remain under services/"
    );
}

#[test]
fn discovery_does_not_change_mcp_mobile_or_prove_identity_via_health() {
    let src = include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/gateway/discovery.rs"
    ));
    for needle in ["/mcp/mobile", "verify_device_token", "/health", "tls_fingerprint"] {
        assert!(
            !src.contains(needle),
            "discovery must not touch {needle} or use it as identity"
        );
    }
}
