use std::io::{Cursor, Read};

use zip::ZipArchive;

use super::*;
use crate::services::cloud_sync::CloudSyncError;
use crate::services::doc_sync::tests::{body, id, Call, FakeCloud, World};
use crate::services::spark_data::DocRow;

fn packed(world: &World, row: &DocRow) -> Vec<u8> {
    let root = workspace_dir(&world.workspaces_root, &world.workspace_id).expect("folder");
    pack_doc(&root, row).expect("pack")
}

fn upload_action(n: u32, version: i64) -> SyncAction {
    SyncAction::Upload { doc_id: id(n), version }
}

fn upload_call(n: u32, version: i64, bytes: usize) -> Call {
    Call::Upload { doc_id: id(n), version, bytes }
}

// --- one document ---

#[test]
fn a_document_goes_up_as_its_package_with_its_version() {
    let world = World::new();
    let row = world.add(1, 4);
    let expected = packed(&world, &row);
    let cloud = FakeCloud::new();

    let outcome = upload_doc(&world.conn, &world.workspaces_root, &cloud, &id(1)).expect("upload");

    assert_eq!(outcome, UploadOutcome::Uploaded { version: 4, bytes: expected.len() });
    assert_eq!(cloud.stored(&id(1)), Some((4, expected.clone())));
    assert_eq!(cloud.calls(), [upload_call(1, 4, expected.len())]);
}

#[test]
fn the_version_is_the_documents_now() {
    let world = World::new();
    world.add(1, 3);
    world.set_version(1, 7);
    let cloud = FakeCloud::new();

    let outcome = upload_doc(&world.conn, &world.workspaces_root, &cloud, &id(1)).expect("upload");

    assert!(matches!(outcome, UploadOutcome::Uploaded { version: 7, .. }), "{outcome:?}");
    assert_eq!(cloud.stored(&id(1)).map(|(version, _)| version), Some(7));
}

#[test]
fn a_new_document_goes_up_at_version_zero() {
    let world = World::new();
    world.add(1, 0);
    let cloud = FakeCloud::new();

    upload_doc(&world.conn, &world.workspaces_root, &cloud, &id(1)).expect("upload");

    assert_eq!(cloud.stored(&id(1)).map(|(version, _)| version), Some(0));
}

#[test]
fn a_document_of_a_disabled_workspace_goes_up_from_its_own_folder() {
    let world = World::new();
    world.add(1, 1);
    world.add_in_disabled_workspace(2, 6);
    let cloud = FakeCloud::new();

    let outcome = upload_doc(&world.conn, &world.workspaces_root, &cloud, &id(2)).expect("upload");

    assert!(matches!(outcome, UploadOutcome::Uploaded { version: 6, .. }), "{outcome:?}");
    let (_, bytes) = cloud.stored(&id(2)).expect("stored");
    let (_, other) = {
        upload_doc(&world.conn, &world.workspaces_root, &cloud, &id(1)).expect("upload");
        cloud.stored(&id(1)).expect("stored")
    };
    assert_ne!(bytes, other, "each package holds its own body");
}

#[test]
fn a_document_that_is_gone_is_not_an_error_and_sends_nothing() {
    let world = World::new();
    let cloud = FakeCloud::new();

    let outcome = upload_doc(&world.conn, &world.workspaces_root, &cloud, &id(1)).expect("upload");

    assert_eq!(outcome, UploadOutcome::Gone);
    assert_eq!(cloud.calls(), []);
}

#[test]
fn a_document_whose_file_is_missing_is_a_pack_error_and_sends_nothing() {
    let world = World::new();
    world.add_without_file(1, 2);
    let cloud = FakeCloud::new();

    let err = upload_doc(&world.conn, &world.workspaces_root, &cloud, &id(1)).unwrap_err();

    match err {
        DocSyncError::Pack { doc_id, message } => {
            assert_eq!(doc_id, id(1));
            assert!(message.contains("topic.md"), "{message}");
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(cloud.calls(), []);
}

#[test]
fn a_workspace_that_has_no_possible_folder_is_a_pack_error() {
    let world = World::new();
    world.add_in_workspace_with_a_bad_id(1);
    let cloud = FakeCloud::new();

    let err = upload_doc(&world.conn, &world.workspaces_root, &cloud, &id(1)).unwrap_err();

    match err {
        DocSyncError::Pack { doc_id, message } => {
            assert_eq!(doc_id, id(1));
            assert!(message.contains("invalid workspace id"), "{message}");
        }
        other => panic!("{other:?}"),
    }
    assert_eq!(cloud.calls(), []);
}

#[test]
fn what_the_cloud_refuses_comes_back_as_it_is() {
    let world = World::new();
    world.add(1, 1);
    let refusal = CloudSyncError::ObjectTooLarge { limit_bytes: 10 };
    let cloud = FakeCloud::new().failing_upload(&id(1), refusal.clone());

    let err = upload_doc(&world.conn, &world.workspaces_root, &cloud, &id(1)).unwrap_err();

    assert_eq!(err, DocSyncError::Cloud(refusal));
}

#[test]
fn a_database_that_cannot_be_read_is_a_database_error() {
    let world = World::new();
    world.conn.execute("DROP TABLE document", []).expect("drop");
    let cloud = FakeCloud::new();

    let err = upload_doc(&world.conn, &world.workspaces_root, &cloud, &id(1)).unwrap_err();

    assert!(matches!(err, DocSyncError::Database(_)), "{err:?}");
    assert_eq!(cloud.calls(), []);
}

// --- the actions of a plan ---

#[test]
fn only_the_upload_actions_are_run_in_order() {
    let world = World::new();
    let first = world.add(1, 5);
    world.add(2, 3);
    let third = world.add(3, 0);
    let cloud = FakeCloud::new().with(&id(2), 3).with(&id(9), 9);
    let actions = [
        upload_action(1, 5),
        SyncAction::Skip { doc_id: id(2) },
        SyncAction::Pull { doc_id: id(9), version: 9 },
        upload_action(3, 0),
    ];

    let report = push_uploads(&world.conn, &world.workspaces_root, &cloud, &actions);

    let (first_bytes, third_bytes) = (packed(&world, &first).len(), packed(&world, &third).len());
    assert_eq!(
        report,
        PushReport {
            uploaded: vec![
                UploadedDoc { doc_id: id(1), version: 5, bytes: first_bytes },
                UploadedDoc { doc_id: id(3), version: 0, bytes: third_bytes },
            ],
            ..PushReport::default()
        }
    );
    assert_eq!(cloud.calls(), [upload_call(1, 5, first_bytes), upload_call(3, 0, third_bytes)]);
    assert_eq!(cloud.stored(&id(9)), Some((9, b"old".to_vec())), "pulls are not run here");
}

#[test]
fn a_plan_without_uploads_does_nothing() {
    let world = World::new();
    let cloud = FakeCloud::new();
    let actions = [SyncAction::Skip { doc_id: id(1) }, SyncAction::Pull { doc_id: id(2), version: 1 }];

    assert_eq!(push_uploads(&world.conn, &world.workspaces_root, &cloud, &actions), PushReport::default());
    assert_eq!(push_uploads(&world.conn, &world.workspaces_root, &cloud, &[]), PushReport::default());
    assert_eq!(cloud.calls(), []);
}

#[test]
fn a_broken_document_is_set_aside_and_the_others_go_up() {
    let world = World::new();
    world.add(1, 1);
    world.add_without_file(2, 1);
    world.add(3, 1);
    let cloud = FakeCloud::new();
    let actions = [upload_action(1, 1), upload_action(2, 1), upload_action(3, 1)];

    let report = push_uploads(&world.conn, &world.workspaces_root, &cloud, &actions);

    let uploaded: Vec<&str> = report.uploaded.iter().map(|d| d.doc_id.as_str()).collect();
    assert_eq!(uploaded, [id(1), id(3)]);
    assert_eq!(report.failed.len(), 1);
    assert_eq!(report.failed[0].0, id(2));
    assert!(matches!(report.failed[0].1, DocSyncError::Pack { .. }), "{:?}", report.failed[0].1);
    assert_eq!(report.stopped, None);
    assert_eq!(report.not_tried, Vec::<String>::new());
}

#[test]
fn a_document_the_cloud_refuses_for_its_own_reason_does_not_stop_the_run() {
    let world = World::new();
    world.add(1, 1);
    world.add(2, 1);
    world.add(3, 1);
    let too_big = CloudSyncError::ObjectTooLarge { limit_bytes: 10 };
    let cloud = FakeCloud::new().failing_upload(&id(2), too_big.clone());
    let actions = [upload_action(1, 1), upload_action(2, 1), upload_action(3, 1)];

    let report = push_uploads(&world.conn, &world.workspaces_root, &cloud, &actions);

    assert_eq!(report.uploaded.len(), 2);
    assert_eq!(report.failed, [(id(2), DocSyncError::Cloud(too_big))]);
    assert_eq!(report.stopped, None);
    assert_eq!(cloud.calls().len(), 3);
}

#[test]
fn an_error_that_would_hit_every_document_stops_the_run_and_names_what_was_left() {
    let world = World::new();
    for n in 1..=4 {
        world.add(n, 1);
    }
    let down = CloudSyncError::Network("down".into());
    let cloud = FakeCloud::new().failing_upload(&id(2), down.clone());
    let actions = [
        upload_action(1, 1),
        upload_action(2, 1),
        SyncAction::Skip { doc_id: id(5) },
        upload_action(3, 1),
        SyncAction::Pull { doc_id: id(6), version: 1 },
        upload_action(4, 1),
    ];

    let report = push_uploads(&world.conn, &world.workspaces_root, &cloud, &actions);

    assert_eq!(report.uploaded.len(), 1);
    assert_eq!(report.uploaded[0].doc_id, id(1));
    assert_eq!(report.stopped, Some((id(2), DocSyncError::Cloud(down))));
    assert_eq!(report.not_tried, [id(3), id(4)], "only upload actions are listed");
    assert_eq!(report.failed, []);
    assert_eq!(cloud.calls().len(), 2, "nothing is sent after the stop");
}

#[test]
fn a_session_that_ran_out_stops_the_run_at_the_first_document() {
    let world = World::new();
    world.add(1, 1);
    world.add(2, 1);
    let cloud = FakeCloud::new()
        .failing_upload(&id(1), CloudSyncError::SessionExpired)
        .failing_upload(&id(2), CloudSyncError::SessionExpired);

    let report = push_uploads(&world.conn, &world.workspaces_root, &cloud, &[upload_action(1, 1), upload_action(2, 1)]);

    assert_eq!(report.stopped, Some((id(1), DocSyncError::Cloud(CloudSyncError::SessionExpired))));
    assert_eq!(report.not_tried, [id(2)]);
    assert_eq!(report.uploaded, []);
}

#[test]
fn a_document_deleted_after_the_plan_is_noted_and_the_run_goes_on() {
    let world = World::new();
    world.add(1, 1);
    world.add(2, 1);
    world.remove_row(1);
    let cloud = FakeCloud::new();

    let report = push_uploads(&world.conn, &world.workspaces_root, &cloud, &[upload_action(1, 1), upload_action(2, 1)]);

    assert_eq!(report.gone, [id(1)]);
    assert_eq!(report.uploaded.len(), 1);
    assert_eq!(report.uploaded[0].doc_id, id(2));
    assert_eq!(cloud.calls().len(), 1);
}

#[test]
fn the_body_that_goes_up_is_the_one_on_disk() {
    let world = World::new();
    world.add(1, 1);
    let cloud = FakeCloud::new();

    push_uploads(&world.conn, &world.workspaces_root, &cloud, &[upload_action(1, 1)]);

    let (_, package) = cloud.stored(&id(1)).expect("stored");
    let mut zip = ZipArchive::new(Cursor::new(package)).expect("a zip");
    let mut sent = Vec::new();
    zip.by_name("raw/notes/202606190001-topic.md").expect("the body").read_to_end(&mut sent).expect("read");
    assert_eq!(sent, body(1));
}
