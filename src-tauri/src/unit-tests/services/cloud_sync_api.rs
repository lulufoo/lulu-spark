use super::*;
use crate::services::cloud_sync::tests::{
    creds_reply, dead_url, link_reply, list_reply, seen, MockWorker, Reply, Rig, DOC_A, DOC_B, JWT, NOW, USER,
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
    let rig = Rig::new(vec![Reply::raw(503, ""), Reply::raw(503, ""), Reply::raw(503, ""), Reply::raw(503, "")]);
    let cloud: &dyn CloudDocs = &rig.sync;

    let unavailable = CloudSyncError::Unavailable { status: 503, code: "unknown".into() };
    assert_eq!(cloud.list_documents().unwrap_err(), unavailable);
    assert_eq!(cloud.upload(DOC_A, b"x", 1).unwrap_err(), unavailable);
    assert_eq!(cloud.read(DOC_A).unwrap_err(), unavailable);
    assert_eq!(cloud.delete(DOC_A).unwrap_err(), unavailable);
}

#[test]
fn the_real_cloud_deletes_through_the_trait() {
    let rig = Rig::new(vec![Reply::bytes(204, b"")]);
    let cloud: &dyn CloudDocs = &rig.sync;

    cloud.delete(DOC_A).expect("delete");

    assert_eq!(rig.worker.seen(), [seen("DELETE", &format!("/documents/{DOC_A}"), JWT)]);
}

// --- delete_cloud_copy: what a user's delete does about the cloud ---

#[test]
fn a_signed_in_delete_removes_the_cloud_copy() {
    let rig = Rig::new(vec![Reply::bytes(204, b"")]);

    delete_cloud_copy(&rig.sync, DOC_B).expect("deleted");

    assert_eq!(rig.worker.seen(), [seen("DELETE", &format!("/documents/{DOC_B}"), JWT)]);
}

#[test]
fn a_document_the_cloud_never_had_is_deleted_all_the_same() {
    // The Worker answers 204 for an object that is not there.
    let rig = Rig::new(vec![Reply::bytes(204, b"")]);

    assert_eq!(delete_cloud_copy(&rig.sync, DOC_A), Ok(()));
}

#[test]
fn nobody_signed_in_has_no_cloud_copy_and_nothing_is_sent() {
    let rig = Rig::new(vec![]);
    rig.sign_out();

    assert_eq!(delete_cloud_copy(&rig.sync, DOC_A), Ok(()));
    assert_eq!(rig.worker.recorded().len(), 0);
}

#[test]
fn an_id_that_cannot_name_a_cloud_object_has_no_cloud_copy_and_nothing_is_sent() {
    let rig = Rig::new(vec![]);
    for id in ["", "d2", "AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA", "../aaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"] {
        assert_eq!(delete_cloud_copy(&rig.sync, id), Ok(()), "{id:?}");
    }
    assert_eq!(rig.worker.recorded().len(), 0);
}

#[test]
fn every_other_failure_is_passed_on_because_the_copy_may_still_be_there() {
    // Each answer is what the real cloud gives for it: the Worker's status, or no answer at all.
    let cases = [
        (Reply::raw(401, ""), CloudSyncError::Unauthorized),
        (Reply::raw(503, ""), CloudSyncError::Unavailable { status: 503, code: "unknown".into() }),
        (Reply::raw(500, ""), CloudSyncError::Unavailable { status: 500, code: "unknown".into() }),
        (Reply::raw(403, ""), CloudSyncError::Rejected { status: 403, code: "unknown".into() }),
        (Reply::raw(302, ""), CloudSyncError::BadResponse("unexpected status 302".into())),
    ];
    for (reply, expected) in cases {
        let rig = Rig::new(vec![reply]);
        assert_eq!(delete_cloud_copy(&rig.sync, DOC_A), Err(expected));
    }

    let rig = Rig::new(vec![]);
    rig.sign_in(USER, JWT, NOW);
    assert_eq!(delete_cloud_copy(&rig.sync, DOC_A), Err(CloudSyncError::SessionExpired));

    let rig = Rig::new(vec![]);
    rig.vault_fails("locked");
    assert!(matches!(delete_cloud_copy(&rig.sync, DOC_A), Err(CloudSyncError::Vault(_))));

    let offline = Rig::with_url(MockWorker::start(vec![]), &dead_url());
    assert!(matches!(delete_cloud_copy(&offline.sync, DOC_A), Err(CloudSyncError::Network(_))));
}
