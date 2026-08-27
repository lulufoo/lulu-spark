//! Current LAN IPv4: live NIC read, no cache, no listen.

use std::net::Ipv4Addr;

#[cfg(test)]
use std::sync::Mutex;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NicIpv4 {
    pub name: String,
    pub addr: Ipv4Addr,
    pub up: bool,
}

#[cfg(test)]
static TEST_NICS: Mutex<Option<Vec<NicIpv4>>> = Mutex::new(None);

#[cfg(test)]
pub fn test_override_nics(nics: Option<Vec<NicIpv4>>) {
    *TEST_NICS.lock().unwrap_or_else(|e| e.into_inner()) = nics;
}

pub fn current_lan_ipv4() -> Option<Ipv4Addr> {
    #[cfg(test)]
    {
        let guard = TEST_NICS.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(nics) = guard.as_ref() {
            return select_lan_ipv4(nics);
        }
    }
    select_lan_ipv4(&enumerate_nics())
}

pub fn select_lan_ipv4(nics: &[NicIpv4]) -> Option<Ipv4Addr> {
    let qualified: Vec<&NicIpv4> = nics.iter().filter(|nic| is_qualified(nic)).collect();
    if let Some(en0) = qualified
        .iter()
        .find(|nic| nic.name == "en0" && is_rfc1918(nic.addr))
    {
        return Some(en0.addr);
    }
    qualified.first().map(|nic| nic.addr)
}

fn is_qualified(nic: &NicIpv4) -> bool {
    nic.up && !nic.addr.is_loopback() && !nic.addr.is_link_local()
}

fn is_rfc1918(addr: Ipv4Addr) -> bool {
    let o = addr.octets();
    o[0] == 10 || (o[0] == 172 && (16..=31).contains(&o[1])) || (o[0] == 192 && o[1] == 168)
}

fn enumerate_nics() -> Vec<NicIpv4> {
    let mut out = Vec::new();
    unsafe {
        let mut raw = std::ptr::null_mut();
        if libc::getifaddrs(&mut raw) != 0 {
            return out;
        }
        let mut ptr = raw;
        while !ptr.is_null() {
            let ifa = &*ptr;
            if !ifa.ifa_name.is_null() && !ifa.ifa_addr.is_null() {
                let family = (*ifa.ifa_addr).sa_family as i32;
                if family == libc::AF_INET {
                    let name = std::ffi::CStr::from_ptr(ifa.ifa_name)
                        .to_string_lossy()
                        .into_owned();
                    let up = (ifa.ifa_flags as libc::c_int & libc::IFF_UP) != 0;
                    let sin = &*(ifa.ifa_addr as *const libc::sockaddr_in);
                    let addr = Ipv4Addr::from(u32::from_be(sin.sin_addr.s_addr));
                    out.push(NicIpv4 { name, addr, up });
                }
            }
            ptr = (*ptr).ifa_next;
        }
        libc::freeifaddrs(raw);
    }
    out
}

#[cfg(test)]
#[path = "../../unit-tests/services/lan_ip.rs"]
mod tests;
