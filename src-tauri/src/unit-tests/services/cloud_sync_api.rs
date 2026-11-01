use super::*;
use crate::services::cloud_sync::tests::{
    creds_reply, link_reply, list_reply, seen, Reply, Rig, DOC_A, DOC_B, JWT, NOW,
};

#[test]
fn the_real_cloud_lists_through_the_trait() {
    let rig = Rig::new(vec![list_reply(&[(DOC_A, 2), (DOC_B, 5)])]);
    let cloud: &dyn CloudDocs = &rig.sync;

    let docs = cloud.list_documents().expect("list");

    assert_eq!(
        docs,
        [
            CloudDoc { document_id: DOC_A.into(), version: 2 },
            CloudDoc { document_id: DOC_B.into(), version: 5 },
        ]
    );
    assert_eq!(rig.worker.seen(), [seen("GET", "/documents", JWT)]);
}

#[test]
fn the_real_cloud_uploads_through_the_trait() {
    let rig = Rig::new(vec![link_reply("/put/one"), Reply::raw(200, "")]);
    let cloud: &dyn CloudDocs = &rig.sync;

    cloud.upload(DOC_B, b"package", 9).expect("upload");

    let requests = rig.worker.recorded();
    assert_eq!(requests.len(), 2);
    assert_eq!(requests[0].path, "/uploads");
    assert_eq!(requests[0].json()["documentId"], DOC_B);
    assert_eq!(requests[0].json()["size"], 7);
    assert_eq!(requests[0].json()["version"], 9);
    assert_eq!(requests[1].method, "PUT");
    assert_eq!(requests[1].body, b"package");
}

#[test]
fn the_real_cloud_reads_through_the_trait() {
    let rig = Rig::new(vec![creds_reply("gcs-1", NOW + 1800), Reply::bytes(200, b"the package")]);
    let cloud: &dyn CloudDocs = &rig.sync;

    let bytes = cloud.read(DOC_A).expect("read");

    assert_eq!(bytes, b"the package");
    let requests = rig.worker.recorded();
    assert_eq!(requests[1].path, format!("/storage/v1/b/spark-abc/o/{DOC_A}?alt=media"));
    assert_eq!(requests[1].header("authorization"), Some("Bearer gcs-1"));
}

#[test]
fn the_errors_come_through_the_trait_unchanged() {
    let rig = Rig::new(vec![Reply::raw(503, ""), Reply::raw(503, ""), Reply::raw(503, "")]);
    let cloud: &dyn CloudDocs = &rig.sync;

    let unavailable = CloudSyncError::Unavailable { status: 503, code: "unknown".into() };
    assert_eq!(cloud.list_documents().unwrap_err(), unavailable);
    assert_eq!(cloud.upload(DOC_A, b"x", 1).unwrap_err(), unavailable);
    assert_eq!(cloud.read(DOC_A).unwrap_err(), unavailable);
}
