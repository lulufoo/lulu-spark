//! Compile-time package snapshot for the running binary.

use serde::Serialize;

pub const VERSION: &str = "Beta v0.1";
pub const PRODUCT_NAME: &str = "Lulu Spark";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PackageSnapshot {
    pub is_debug: bool,
    pub version: &'static str,
    pub product_name: &'static str,
}

pub fn snapshot() -> PackageSnapshot {
    PackageSnapshot {
        is_debug: cfg!(debug_assertions),
        version: VERSION,
        product_name: PRODUCT_NAME,
    }
}

#[cfg(test)]
#[path = "../unit-tests/host/package.rs"]
mod tests;
