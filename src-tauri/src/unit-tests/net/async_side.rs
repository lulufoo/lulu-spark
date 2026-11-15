//! The 0.13 side builds from the same plan and leaves the same evidence.

use std::time::Duration;

use super::kernel as kernel_tests;
use super::*;
use crate::net::ClientSpec;

#[tokio::test]
async fn async_client_routes_through_the_current_plan() {
    let _lock = lock_env();
    let _clean = EnvCleaner;
    let proxy = kernel_tests::ObservableProxy::start();
    let kernel = kernel_with(true, snapshot(Some(("127.0.0.1", Some(proxy.port))), None, &[]));
    let spec = ClientSpec::default().with_request_timeout(Duration::from_secs(5));
    let client = kernel.async_external(&spec).expect("async client");
    let response = client
        .get("http://spark.test/echo")
        .send()
        .await
        .expect("request must travel through the proxy");
    assert_eq!(response.status().as_u16(), 200);
    let body = response.text().await.expect("body");
    assert_eq!(body, "proxied-hit");
    let hits = proxy.hits();
    assert_eq!(hits, vec!["GET http://spark.test/echo".to_string()]);
}
