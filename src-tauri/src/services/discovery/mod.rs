//! Publish current LAN IPv4 + Gateway port via system mDNS/Bonjour.

use std::ffi::CString;
use std::net::Ipv4Addr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::services::lan_ip::current_lan_ipv4;

pub const SERVICE_TYPE: &str = "_lulu-workbench._tcp";
pub const DEFAULT_REFRESH_INTERVAL: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdvertisedRecord {
    pub service_type: String,
    pub ipv4: Ipv4Addr,
    pub port: u16,
}

impl AdvertisedRecord {
    pub fn txt_pairs(&self) -> Vec<(String, String)> {
        Vec::new()
    }
}

pub trait Publisher: Send + 'static {
    fn publish(&mut self, record: AdvertisedRecord) -> Result<(), String>;
    fn withdraw(&mut self);
}

#[derive(Debug)]
pub enum DiscoveryError {
    PublishFailed(String),
}

impl std::fmt::Display for DiscoveryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::PublishFailed(msg) => write!(f, "{msg}"),
        }
    }
}

pub struct DiscoveryConfig {
    pub gateway_port: u16,
    pub refresh_interval: Duration,
}

impl DiscoveryConfig {
    pub fn for_gateway_port(gateway_port: u16) -> Self {
        Self {
            gateway_port,
            refresh_interval: DEFAULT_REFRESH_INTERVAL,
        }
    }
}

struct Session {
    publisher: Box<dyn Publisher>,
    published: Option<AdvertisedRecord>,
    gateway_port: u16,
}

impl Session {
    fn apply(&mut self) {
        match current_lan_ipv4() {
            Some(ipv4) => {
                let record = AdvertisedRecord {
                    service_type: SERVICE_TYPE.to_string(),
                    ipv4,
                    port: self.gateway_port,
                };
                if self.published.as_ref() == Some(&record) {
                    return;
                }
                if let Err(err) = self.publisher.publish(record.clone()) {
                    eprintln!("[discovery] publish failed: {err}");
                    self.published = None;
                    return;
                }
                self.published = Some(record);
            }
            None => self.withdraw_if_published(),
        }
    }

    fn withdraw_if_published(&mut self) {
        if self.published.take().is_some() {
            self.publisher.withdraw();
        }
    }
}

pub struct DiscoveryHandle {
    session: Arc<Mutex<Session>>,
    stop: Arc<AtomicBool>,
    wake: Arc<(Mutex<()>, Condvar)>,
    join: Option<JoinHandle<()>>,
}

fn apply(session: &Arc<Mutex<Session>>) {
    if let Ok(mut inner) = session.lock() {
        inner.apply();
    }
}

pub fn start_with<P: Publisher>(config: DiscoveryConfig, publisher: P) -> DiscoveryHandle {
    let session = Arc::new(Mutex::new(Session {
        publisher: Box::new(publisher),
        published: None,
        gateway_port: config.gateway_port,
    }));
    apply(&session);
    let stop = Arc::new(AtomicBool::new(false));
    let wake = Arc::new((Mutex::new(()), Condvar::new()));
    let session_thread = Arc::clone(&session);
    let stop_thread = Arc::clone(&stop);
    let wake_thread = Arc::clone(&wake);
    let interval = config.refresh_interval;
    let join = thread::spawn(move || {
        while !stop_thread.load(Ordering::SeqCst) {
            let (lock, cvar) = &*wake_thread;
            let guard = lock.lock().unwrap_or_else(|e| e.into_inner());
            let wait = cvar.wait_timeout_while(guard, interval, |_| {
                !stop_thread.load(Ordering::SeqCst)
            });
            drop(match wait {
                Ok((guard, _)) => guard,
                Err(poisoned) => poisoned.into_inner().0,
            });
            if stop_thread.load(Ordering::SeqCst) {
                break;
            }
            apply(&session_thread);
        }
    });
    DiscoveryHandle {
        session,
        stop,
        wake,
        join: Some(join),
    }
}

pub fn refresh(handle: &DiscoveryHandle) {
    apply(&handle.session);
}

pub fn current_advertisement(handle: &DiscoveryHandle) -> Option<AdvertisedRecord> {
    handle
        .session
        .lock()
        .ok()
        .and_then(|inner| inner.published.clone())
}

pub fn stop(handle: DiscoveryHandle) {
    drop(handle);
}

impl Drop for DiscoveryHandle {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        self.wake.1.notify_all();
        if let Ok(mut inner) = self.session.lock() {
            inner.withdraw_if_published();
        }
        if let Some(join) = self.join.take() {
            let _ = join.join();
        }
    }
}

type DNSServiceRef = *mut libc::c_void;

// System mDNS/Bonjour (libSystem / dns_sd), not a homemade UDP responder.
extern "C" {
    fn DNSServiceRegister(
        sd_ref: *mut DNSServiceRef,
        flags: u32,
        interface_index: u32,
        name: *const libc::c_char,
        regtype: *const libc::c_char,
        domain: *const libc::c_char,
        host: *const libc::c_char,
        port: u16,
        txt_len: u16,
        txt_record: *const libc::c_void,
        callback: *const libc::c_void,
        context: *mut libc::c_void,
    ) -> i32;
    fn DNSServiceRefDeallocate(sd_ref: DNSServiceRef);
}

struct BonjourPublisher {
    service: Option<DNSServiceRef>,
}

impl BonjourPublisher {
    fn new() -> Self {
        Self { service: None }
    }
}

// SAFETY: DNSServiceRef is moved only with the publisher, which stays on one thread.
unsafe impl Send for BonjourPublisher {}

impl Publisher for BonjourPublisher {
    fn publish(&mut self, record: AdvertisedRecord) -> Result<(), String> {
        self.withdraw();
        let name = CString::new("lulu-workbench").map_err(|err| err.to_string())?;
        let regtype = CString::new(SERVICE_TYPE).map_err(|err| err.to_string())?;
        let host = CString::new(record.ipv4.to_string()).map_err(|err| err.to_string())?;
        let mut sd_ref: DNSServiceRef = std::ptr::null_mut();
        let err = unsafe {
            DNSServiceRegister(
                &mut sd_ref,
                0,
                0,
                name.as_ptr(),
                regtype.as_ptr(),
                std::ptr::null(),
                host.as_ptr(),
                record.port.to_be(),
                0,
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null_mut(),
            )
        };
        if err != 0 || sd_ref.is_null() {
            return Err(format!("DNSServiceRegister failed: {err}"));
        }
        self.service = Some(sd_ref);
        Ok(())
    }

    fn withdraw(&mut self) {
        if let Some(sd_ref) = self.service.take() {
            unsafe {
                DNSServiceRefDeallocate(sd_ref);
            }
        }
    }
}

impl Drop for BonjourPublisher {
    fn drop(&mut self) {
        self.withdraw();
    }
}

pub fn start(config: DiscoveryConfig) -> Result<DiscoveryHandle, DiscoveryError> {
    Ok(start_with(config, BonjourPublisher::new()))
}

pub struct DiscoveryState {
    handle: Mutex<Option<DiscoveryHandle>>,
}

impl DiscoveryState {
    pub fn new() -> Self {
        Self {
            handle: Mutex::new(None),
        }
    }

    pub fn try_start(&self, gateway_port: u16) {
        match start(DiscoveryConfig::for_gateway_port(gateway_port)) {
            Ok(handle) => {
                if let Ok(mut guard) = self.handle.lock() {
                    if let Some(prev) = guard.take() {
                        stop(prev);
                    }
                    *guard = Some(handle);
                }
            }
            Err(err) => {
                eprintln!("[discovery] start failed: {err}");
            }
        }
    }

    pub fn stop(&self) {
        if let Ok(mut guard) = self.handle.lock() {
            if let Some(handle) = guard.take() {
                stop(handle);
            }
        }
    }
}

#[cfg(test)]
#[path = "../../unit-tests/services/discovery.rs"]
mod tests;
