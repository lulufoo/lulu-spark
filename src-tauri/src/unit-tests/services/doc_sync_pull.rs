use std::collections::BTreeMap;

use serde_json::json;

use super::*;
use crate::services::cloud_sync::CloudSyncError;
use crate::services::doc_sync::{compare, push_uploads};
use crate::services::doc_sync::tests::{
    attachment_body, body, digest_body, id, translation_body, write_at, Call, FakeCloud, World,
};
use crate::services::spark_data::{find_doc, get_doc, insert_project, update_doc, ProjectRow};

const RAW_1: &str = "notes/202606190001-topic.md";
const EN_1: &str = "notes/202606190001-topic-en.md";

/// A world that holds document 1 in full at `version`, with a body of its own, and the package
/// that world would upload.
fn source(version: i64) -> (World, Vec<u8>) {
    let world = World::new();
    world.add_full(1, version);
    write_at(&world.path("raw", RAW_1), b"the body in the cloud");
    let package = world.package_of(1);
    (world, package)
}

fn pull(world: &World, cloud: &FakeCloud, n: u32, version: i64) -> Result<PullOutcome, DocSyncError> {
    pull_doc(&world.conn, &world.workspaces_root, cloud, &id(n), version)
}

fn pull_suffixed(world: &World, cloud: &FakeCloud, n: u32, version: i64) -> Result<PullOutcome, DocSyncError> {
    pull_with(&world.conn, &world.workspaces_root, cloud, &id(n), version, || "sfx".to_string())
}

fn pull_action(n: u32, version: i64) -> SyncAction {
    SyncAction::Pull { doc_id: id(n), version }
}

fn read_call(n: u32) -> Call {
    Call::Read { doc_id: id(n) }
}

/// Edits the row the way a user does (title, tags, …), adds a translation and an attachment the
/// cloud's package does not have, and returns the row as it is now.
fn customise(world: &World, n: u32) -> DocRow {
    let mut row = world.row_of(n).expect("row");
    row.title = Some("Mine".into());
    row.tags = json!(["a", "b"]);
    row.done = true;
    row.importance = Some("high".into());
    row.links = json!(["l"]);
    row.comments = json!(["c"]);
    row.highlights = json!(["h"]);
    row.created_at = Some("202606190001".into());
    row.source_type = Some("web".into());
    row.translations = json!(["en", "fr"]);
    assert!(update_doc(&world.conn, &row).expect("update"));
    let fr = format!("notes/2026061900{n:02}-topic-fr.md");
    write_at(&world.path("raw", &fr), b"only here");
    write_at(&world.path("raw", &format!("notes/.attached/{}/only-here.png", id(n))), b"only here");
    world.row_of(n).expect("row")
}

// --- a document that is not here ---

#[test]
fn a_document_that_is_missing_here_comes_into_the_inbox_with_default_columns() {
    let (cloud_world, package) = source(3);
    let world = World::new(); // no workspace folder yet
    let cloud = FakeCloud::new().with_package(&id(1), 3, package.clone());

    let outcome = pull(&world, &cloud, 1, 3).expect("pull");

    assert_eq!(
        outcome,
        PullOutcome::Pulled { version: 3, bytes: package.len(), created: true, workspace_id: world.workspace_id.clone() }
    );
    let mut expected = DocRow::new(&id(1), RAW_1, &world.project_id);
    expected.digest_path = Some(RAW_1.to_string());
    expected.translations = json!(["en"]);
    expected.version = 3;
    assert_eq!(world.row_of(1), Some(expected));
    assert_eq!(world.inbox_files(), cloud_world.inbox_files());
}

#[test]
fn a_new_document_takes_the_version_the_cloud_gave_never_plus_one() {
    for version in [0, 1, 7, 1_000_000] {
        let (_cloud_world, package) = source(version);
        let world = World::new();
        let cloud = FakeCloud::new().with_package(&id(1), version, package);

        pull(&world, &cloud, 1, version).expect("pull");

        assert_eq!(world.row_of(1).map(|row| row.version), Some(version));
    }
}

#[test]
fn a_new_document_has_the_translations_the_package_has() {
    let world = World::new();
    world.add_full(1, 1);
    let mut row = world.row_of(1).expect("row");
    write_at(&world.path("raw", "notes/202606190001-topic-zh.md"), "中文".as_bytes());
    row.translations = json!(["zh", "en"]);
    assert!(update_doc(&world.conn, &row).expect("update"));
    let package = world.package_of(1);
    let elsewhere = World::new();
    let cloud = FakeCloud::new().with_package(&id(1), 1, package);

    pull(&elsewhere, &cloud, 1, 1).expect("pull");

    assert_eq!(elsewhere.row_of(1).map(|row| row.translations), Some(json!(["zh", "en"])));
    assert_eq!(elsewhere.inbox_files(), world.inbox_files());
}

#[test]
fn a_new_document_goes_to_the_inbox_of_the_default_workspace_even_with_other_workspaces_around() {
    let (_cloud_world, package) = source(2);
    let world = World::new();
    world.add_in_disabled_workspace(5, 1);
    let cloud = FakeCloud::new().with_package(&id(1), 2, package);

    pull(&world, &cloud, 1, 2).expect("pull");

    let found = find_doc(&world.conn, &id(1)).expect("query").expect("found");
    assert_eq!(found.workspace.id, world.workspace_id);
    assert_eq!(found.doc.project_id, world.project_id);
}

#[test]
fn a_name_in_use_in_the_inbox_gets_a_suffix_and_the_whole_document_follows() {
    let (_cloud_world, package) = source(2);
    let world = World::new();
    world.add_at(2, 1, RAW_1); // another document has the body path
    let cloud = FakeCloud::new().with_package(&id(1), 2, package);

    pull_suffixed(&world, &cloud, 1, 2).expect("pull");

    let renamed = "notes/202606190001-topic-sfx.md";
    let row = world.row_of(1).expect("row");
    assert_eq!(row.raw_path, renamed);
    assert_eq!(row.digest_path.as_deref(), Some(renamed));
    assert_eq!(row.translations, json!(["en"]));
    let mut expected = BTreeMap::new();
    expected.insert(format!("raw/{renamed}"), b"the body in the cloud".to_vec());
    expected.insert("raw/notes/202606190001-topic-sfx-en.md".to_string(), translation_body(1));
    expected.insert(format!("digest/{renamed}"), digest_body(1));
    expected.insert(format!("raw/notes/.attached/{}/figure.png", id(1)), attachment_body(1));
    expected.insert(format!("raw/{RAW_1}"), body(2));
    assert_eq!(world.inbox_files(), expected);
    assert_eq!(world.row_of(2).map(|row| row.raw_path), Some(RAW_1.to_string()), "the other row is untouched");
}

#[test]
fn a_name_used_in_another_project_is_free_in_the_inbox() {
    let (cloud_world, package) = source(2);
    let world = World::new();
    insert_project(&world.conn, &ProjectRow::new("other1", &world.workspace_id, "Other", "")).expect("project");
    world.add_in_project_at("other1", 2, 1, RAW_1);
    let cloud = FakeCloud::new().with_package(&id(1), 2, package);

    pull_suffixed(&world, &cloud, 1, 2).expect("pull");

    assert_eq!(world.row_of(1).map(|row| row.raw_path), Some(RAW_1.to_string()), "no suffix: the name is free here");
    assert_eq!(world.inbox_files(), cloud_world.inbox_files());
    assert_eq!(world.files_of("other1").get(&format!("raw/{RAW_1}")), Some(&body(2)), "the other project is untouched");
}

#[test]
fn a_file_on_disk_that_no_row_names_is_not_overwritten() {
    let (_cloud_world, package) = source(2);
    let world = World::new();
    write_at(&world.path("raw", EN_1), b"a stray file");
    let cloud = FakeCloud::new().with_package(&id(1), 2, package);

    pull_suffixed(&world, &cloud, 1, 2).expect("pull");

    assert_eq!(world.row_of(1).map(|row| row.raw_path), Some("notes/202606190001-topic-sfx.md".to_string()));
    assert_eq!(std::fs::read(world.path("raw", EN_1)).expect("stray"), b"a stray file");
}

#[test]
fn a_name_that_cannot_be_made_free_fails_and_changes_nothing() {
    let (_cloud_world, package) = source(2);
    let world = World::new();
    world.add_at(2, 1, RAW_1);
    write_at(&world.path("raw", "notes/202606190001-topic-sfx.md"), b"in the way");
    let before = world.inbox_files();
    let cloud = FakeCloud::new().with_package(&id(1), 2, package);

    let err = pull_suffixed(&world, &cloud, 1, 2).unwrap_err();

    assert!(matches!(&err, DocSyncError::Unpack { doc_id, message } if *doc_id == id(1) && message.contains("no free name")), "{err:?}");
    assert_eq!(world.inbox_files(), before);
    assert_eq!(world.row_of(1), None);
}

// --- a document that is here and behind ---

#[test]
fn a_document_that_is_behind_gets_the_files_of_the_package_and_nothing_else() {
    let (cloud_world, package) = source(5);
    let world = World::new();
    world.add_full(1, 2);
    let before = customise(&world, 1);
    let cloud = FakeCloud::new().with_package(&id(1), 5, package.clone());

    let outcome = pull(&world, &cloud, 1, 5).expect("pull");

    assert_eq!(
        outcome,
        PullOutcome::Pulled { version: 5, bytes: package.len(), created: false, workspace_id: world.workspace_id.clone() }
    );
    // The translation and the attachment only this machine had are gone: the files are the cloud's.
    assert_eq!(world.inbox_files(), cloud_world.inbox_files());
    // Every column but the translations and the version is what it was.
    let mut expected = before;
    expected.translations = json!(["en"]);
    expected.version = 5;
    assert_eq!(world.row_of(1), Some(expected));
}

#[test]
fn the_version_of_a_document_that_was_behind_is_the_cloud_s_exactly() {
    for (local, cloud_version) in [(0, 1), (3, 4), (3, 10), (0, 1_000_000)] {
        let (_cloud_world, package) = source(cloud_version);
        let world = World::new();
        world.add_full(1, local);
        let cloud = FakeCloud::new().with_package(&id(1), cloud_version, package);

        pull(&world, &cloud, 1, cloud_version).expect("pull");

        assert_eq!(world.row_of(1).map(|row| row.version), Some(cloud_version), "from {local}");
    }
}

#[test]
fn a_package_that_moves_the_body_moves_the_row_with_it() {
    let world = World::new();
    world.add_at(1, 1, "notes/202606190009-other-name.md");
    let (cloud_world, package) = source(4);
    let cloud = FakeCloud::new().with_package(&id(1), 4, package);

    pull(&world, &cloud, 1, 4).expect("pull");

    let row = world.row_of(1).expect("row");
    assert_eq!((row.raw_path.as_str(), row.digest_path.as_deref()), (RAW_1, Some(RAW_1)));
    assert_eq!(world.inbox_files(), cloud_world.inbox_files(), "the old body is gone");
}

#[test]
fn a_document_stays_in_its_own_project() {
    let (cloud_world, package) = source(4);
    let world = World::new();
    insert_project(&world.conn, &ProjectRow::new("other1", &world.workspace_id, "Other", "")).expect("project");
    world.add_in_project("other1", 1, 1);
    let cloud = FakeCloud::new().with_package(&id(1), 4, package);

    pull(&world, &cloud, 1, 4).expect("pull");

    assert_eq!(world.row_of(1).map(|row| row.project_id), Some("other1".to_string()));
    assert_eq!(world.files_of("other1"), cloud_world.inbox_files());
    assert_eq!(world.inbox_files(), BTreeMap::new(), "nothing lands in the inbox");
}

#[test]
fn a_document_of_a_disabled_workspace_is_replaced_in_its_own_workspace() {
    let (_cloud_world, package) = source(4);
    let world = World::new();
    world.add_in_disabled_workspace(1, 1);
    let cloud = FakeCloud::new().with_package(&id(1), 4, package);

    let outcome = pull(&world, &cloud, 1, 4).expect("pull");

    let found = find_doc(&world.conn, &id(1)).expect("query").expect("found");
    assert_ne!(found.workspace.id, world.workspace_id);
    assert!(matches!(&outcome, PullOutcome::Pulled { workspace_id, created: false, .. } if *workspace_id == found.workspace.id), "{outcome:?}");
    let own = world.workspaces_root.join(&found.workspace.id).join("raw").join(&found.doc.project_id).join(RAW_1);
    assert_eq!(std::fs::read(own).expect("body"), b"the body in the cloud");
    assert_eq!(found.doc.version, 4);
    assert_eq!(world.inbox_files(), BTreeMap::new());
}

#[test]
fn a_document_that_is_here_keeps_its_name_when_the_name_is_its_own() {
    let (_cloud_world, package) = source(4);
    let world = World::new();
    world.add_full(1, 1);
    let cloud = FakeCloud::new().with_package(&id(1), 4, package);

    pull_suffixed(&world, &cloud, 1, 4).expect("pull");

    assert_eq!(world.row_of(1).map(|row| row.raw_path), Some(RAW_1.to_string()), "no suffix on its own name");
}

// --- a document that has caught up ---

#[test]
fn a_document_that_is_not_behind_any_more_is_left_alone() {
    // (local version, the cloud version the plan saw)
    for (local, cloud_version) in [(5, 5), (6, 5), (9, 0)] {
        let (_cloud_world, package) = source(cloud_version);
        let world = World::new();
        world.add_full(1, local);
        let before_files = world.inbox_files();
        let before_row = world.row_of(1);
        let cloud = FakeCloud::new().with_package(&id(1), cloud_version, package);

        let outcome = pull(&world, &cloud, 1, cloud_version).expect("pull");

        assert_eq!(outcome, PullOutcome::Kept, "local {local}, cloud {cloud_version}");
        assert_eq!(world.inbox_files(), before_files);
        assert_eq!(world.row_of(1), before_row);
    }
}

#[test]
fn a_document_one_version_behind_is_pulled() {
    let (_cloud_world, package) = source(5);
    let world = World::new();
    world.add_full(1, 4);
    let cloud = FakeCloud::new().with_package(&id(1), 5, package);

    assert!(matches!(pull(&world, &cloud, 1, 5), Ok(PullOutcome::Pulled { version: 5, .. })));
}

// --- what goes wrong ---

#[test]
fn a_cloud_that_cannot_be_read_changes_nothing() {
    let errors = [
        CloudSyncError::NotSignedIn,
        CloudSyncError::Unauthorized,
        CloudSyncError::Network("down".into()),
        CloudSyncError::ObjectNotFound,
        CloudSyncError::StorageTokenRejected,
    ];
    for error in errors {
        let world = World::new();
        world.add_full(1, 1);
        let before = world.inbox_files();
        let cloud = FakeCloud::new().failing_read(&id(1), error.clone());

        let got = pull(&world, &cloud, 1, 9);

        assert_eq!(got, Err(DocSyncError::Cloud(error.clone())), "{error:?}");
        assert_eq!(world.inbox_files(), before);
        assert_eq!(world.row_of(1).map(|row| row.version), Some(1));
    }
}

#[test]
fn a_package_that_is_no_good_changes_nothing_here_and_creates_nothing() {
    let other_document = {
        let elsewhere = World::new();
        elsewhere.add_full(2, 3);
        elsewhere.package_of(2)
    };
    let bad: [(&str, Vec<u8>); 3] = [
        ("not a zip", b"this is not a package".to_vec()),
        ("empty", Vec::new()),
        ("the package of another document", other_document),
    ];
    for (what, package) in bad {
        // The document is here: its files and its row stay as they were.
        let world = World::new();
        world.add_full(1, 1);
        let before = (world.inbox_files(), world.row_of(1));
        let cloud = FakeCloud::new().with_package(&id(1), 9, package.clone());

        let err = pull(&world, &cloud, 1, 9).unwrap_err();

        assert!(matches!(&err, DocSyncError::Unpack { doc_id, .. } if *doc_id == id(1)), "{what}: {err:?}");
        assert_eq!((world.inbox_files(), world.row_of(1)), before, "{what}");

        // The document is not here: no row, no file.
        let world = World::new();
        let cloud = FakeCloud::new().with_package(&id(1), 9, package);

        assert!(matches!(pull(&world, &cloud, 1, 9), Err(DocSyncError::Unpack { .. })), "{what}");
        assert_eq!((world.inbox_files(), world.row_of(1)), (BTreeMap::new(), None), "{what}");
    }
}

#[test]
fn a_new_document_whose_row_cannot_be_written_leaves_no_files() {
    let (_cloud_world, package) = source(3);
    let world = World::new();
    world
        .conn
        .execute_batch("CREATE TRIGGER no_inserts BEFORE INSERT ON document BEGIN SELECT RAISE(ABORT, 'no inserts'); END")
        .expect("trigger");
    let cloud = FakeCloud::new().with_package(&id(1), 3, package);

    let err = pull(&world, &cloud, 1, 3).unwrap_err();

    assert!(matches!(&err, DocSyncError::Database(message) if message.contains("no inserts")), "{err:?}");
    assert_eq!(world.inbox_files(), BTreeMap::new());
    assert_eq!(world.row_of(1), None);
}

#[test]
fn a_row_that_cannot_be_updated_is_one_pull_behind_and_the_next_pull_finishes_the_job() {
    let (cloud_world, package) = source(5);
    let world = World::new();
    world.add_full(1, 2);
    world
        .conn
        .execute_batch("CREATE TRIGGER no_updates BEFORE UPDATE ON document BEGIN SELECT RAISE(ABORT, 'no updates'); END")
        .expect("trigger");
    let cloud = FakeCloud::new().with_package(&id(1), 5, package);

    let err = pull(&world, &cloud, 1, 5).unwrap_err();

    assert!(matches!(&err, DocSyncError::Database(message) if message.contains("no updates")), "{err:?}");
    assert_eq!(world.row_of(1).map(|row| row.version), Some(2), "the row still says the old version");
    assert_eq!(world.inbox_files(), cloud_world.inbox_files(), "the files are the package's, not lost");

    world.conn.execute_batch("DROP TRIGGER no_updates").expect("drop");
    assert!(matches!(pull(&world, &cloud, 1, 5), Ok(PullOutcome::Pulled { version: 5, .. })));
    assert_eq!(world.row_of(1).map(|row| row.version), Some(5));
}

#[test]
fn a_document_deleted_while_it_was_pulled_does_not_come_back_as_files() {
    let (_cloud_world, package) = source(5);
    let world = World::new();
    world.add_full(1, 2);
    // The delete lands between reading the row and updating it.
    world
        .conn
        .execute_batch(
            "CREATE TRIGGER vanishes BEFORE UPDATE ON document
             BEGIN DELETE FROM document WHERE doc_id = OLD.doc_id; END",
        )
        .expect("trigger");
    let cloud = FakeCloud::new().with_package(&id(1), 5, package);

    let err = pull(&world, &cloud, 1, 5).unwrap_err();

    assert!(matches!(&err, DocSyncError::Database(message) if message.contains("deleted during the pull")), "{err:?}");
    assert_eq!(world.row_of(1), None);
    assert_eq!(world.inbox_files(), BTreeMap::new());
}

// --- many documents ---

#[test]
fn only_the_pull_actions_are_run_and_in_order() {
    let (_world_1, package_1) = source(2);
    let world = World::new();
    let elsewhere = World::new();
    elsewhere.add_full(2, 6);
    let cloud = FakeCloud::new().with_package(&id(1), 2, package_1).with_package(&id(2), 6, elsewhere.package_of(2));
    let actions = [
        SyncAction::Upload { doc_id: id(7), version: 1 },
        pull_action(2, 6),
        SyncAction::Skip { doc_id: id(8) },
        pull_action(1, 2),
    ];

    let report = pull_downloads(&world.conn, &world.workspaces_root, &cloud, &actions);

    assert_eq!(cloud.calls(), [read_call(2), read_call(1)]);
    let pulled: Vec<(&str, i64, bool)> = report.pulled.iter().map(|d| (d.doc_id.as_str(), d.version, d.created)).collect();
    assert_eq!(pulled, [(id(2).as_str(), 6, true), (id(1).as_str(), 2, true)]);
    assert!(report.pulled.iter().all(|d| d.workspace_id == world.workspace_id && d.bytes > 0));
    assert!(report.kept.is_empty() && report.failed.is_empty() && report.stopped.is_none() && report.not_tried.is_empty());
}

#[test]
fn no_pull_actions_make_an_empty_report() {
    let world = World::new();
    let cloud = FakeCloud::new();
    let actions = [SyncAction::Upload { doc_id: id(1), version: 1 }, SyncAction::Skip { doc_id: id(2) }];

    assert_eq!(pull_downloads(&world.conn, &world.workspaces_root, &cloud, &actions), PullReport::default());
    assert_eq!(cloud.calls(), []);
}

#[test]
fn a_document_that_fails_is_recorded_and_the_run_goes_on() {
    let (_world, package) = source(2);
    let world = World::new();
    let cloud = FakeCloud::new()
        .with_package(&id(1), 2, b"not a package".to_vec())
        .with_package(&id(2), 2, package.clone())
        .with_package(&id(3), 2, {
            let elsewhere = World::new();
            elsewhere.add_full(3, 2);
            elsewhere.package_of(3)
        });
    // The package under document 2 is the package of document 1, and says so: refused.
    let actions = [pull_action(1, 2), pull_action(2, 2), pull_action(3, 2)];

    let report = pull_downloads(&world.conn, &world.workspaces_root, &cloud, &actions);

    let failed: Vec<&str> = report.failed.iter().map(|(doc_id, _)| doc_id.as_str()).collect();
    assert_eq!(failed, [id(1).as_str(), id(2).as_str()]);
    assert!(report.failed.iter().all(|(_, e)| matches!(e, DocSyncError::Unpack { .. })));
    assert_eq!(report.pulled.iter().map(|d| d.doc_id.clone()).collect::<Vec<_>>(), [id(3)]);
    assert!(report.stopped.is_none() && report.not_tried.is_empty());
    assert_eq!(world.row_of(1), None);
    assert_eq!(world.row_of(2), None);
}

#[test]
fn a_document_that_was_deleted_in_the_cloud_meanwhile_does_not_stop_the_run() {
    let (_world, package) = source(2);
    let world = World::new();
    let cloud = FakeCloud::new().with_package(&id(1), 2, package); // document 2 is not there any more

    let report = pull_downloads(&world.conn, &world.workspaces_root, &cloud, &[pull_action(2, 4), pull_action(1, 2)]);

    assert_eq!(report.failed, [(id(2), DocSyncError::Cloud(CloudSyncError::ObjectNotFound))]);
    assert_eq!(report.pulled.len(), 1);
    assert!(report.stopped.is_none());
}

#[test]
fn an_error_that_would_hit_every_document_stops_the_run() {
    let (_world, package) = source(2);
    let world = World::new();
    let cloud = FakeCloud::new()
        .with_package(&id(1), 2, package)
        .failing_read(&id(2), CloudSyncError::Unauthorized);
    let actions = [pull_action(1, 2), pull_action(2, 2), pull_action(3, 2), SyncAction::Skip { doc_id: id(4) }, pull_action(5, 2)];

    let report = pull_downloads(&world.conn, &world.workspaces_root, &cloud, &actions);

    assert_eq!(report.pulled.len(), 1);
    assert_eq!(report.stopped, Some((id(2), DocSyncError::Cloud(CloudSyncError::Unauthorized))));
    assert_eq!(report.not_tried, [id(3), id(5)]);
    assert_eq!(cloud.calls(), [read_call(1), read_call(2)], "nothing is read after the stop");
}

#[test]
fn a_document_that_caught_up_is_listed_as_kept() {
    let (_world, package) = source(3);
    let world = World::new();
    world.add_full(1, 3);
    let cloud = FakeCloud::new().with_package(&id(1), 3, package);

    let report = pull_downloads(&world.conn, &world.workspaces_root, &cloud, &[pull_action(1, 3)]);

    assert_eq!(report.kept, [id(1)]);
    assert!(report.pulled.is_empty() && report.failed.is_empty());
}

#[test]
fn every_pull_action_ends_in_exactly_one_place() {
    let elsewhere = World::new();
    for n in [1, 2, 3] {
        elsewhere.add_full(n, 4);
    }
    let world = World::new();
    world.add_full(2, 4); // caught up
    let cloud = FakeCloud::new()
        .with_package(&id(1), 4, elsewhere.package_of(1)) // pulled
        .with_package(&id(2), 4, elsewhere.package_of(2)) // kept
        .with_package(&id(3), 4, b"junk".to_vec()) // failed
        .failing_read(&id(4), CloudSyncError::Network("down".into())) // stops
        .with_package(&id(5), 4, elsewhere.package_of(1)); // not tried
    let actions: Vec<SyncAction> = (1..=5).map(|n| pull_action(n, 4)).collect();

    let report = pull_downloads(&world.conn, &world.workspaces_root, &cloud, &actions);

    let placed = report.pulled.len() + report.kept.len() + report.failed.len() + report.not_tried.len()
        + usize::from(report.stopped.is_some());
    assert_eq!(placed, actions.len(), "{report:?}");
    assert_eq!((report.pulled.len(), report.kept.len(), report.failed.len(), report.not_tried.len()), (1, 1, 1, 1));
}

// --- the cycle ---

#[test]
fn a_database_with_nothing_in_it_receives_every_document_of_the_cloud_and_then_skips_them() {
    let elsewhere = World::new();
    elsewhere.add_full(1, 4);
    elsewhere.add(2, 0);
    elsewhere.add_full(3, 9);
    let cloud = FakeCloud::new();
    let first = compare(&elsewhere.conn, &cloud).expect("compare");
    let report = push_uploads(&elsewhere.conn, &elsewhere.workspaces_root, &cloud, &first);
    assert_eq!(report.uploaded.len(), 3);

    let world = World::new();
    let plan = compare(&world.conn, &cloud).expect("compare");
    assert_eq!(plan, [pull_action(1, 4), pull_action(2, 0), pull_action(3, 9)]);
    let report = pull_downloads(&world.conn, &world.workspaces_root, &cloud, &plan);

    assert_eq!(report.pulled.len(), 3, "{report:?}");
    assert_eq!(world.inbox_files(), elsewhere.inbox_files());
    for n in [1, 2, 3] {
        let (here, there) = (world.row_of(n).expect("pulled"), elsewhere.row_of(n).expect("source"));
        assert_eq!((here.version, here.raw_path, here.digest_path, here.translations),
                   (there.version, there.raw_path, there.digest_path, there.translations));
        assert_eq!(here.project_id, world.project_id);
    }
    let again = compare(&world.conn, &cloud).expect("compare");
    assert!(again.iter().all(|a| matches!(a, SyncAction::Skip { .. })) && again.len() == 3, "{again:?}");
    // …and what was pulled does not go back up.
    assert!(push_uploads(&world.conn, &world.workspaces_root, &cloud, &again).uploaded.is_empty());
}

#[test]
fn a_document_that_is_behind_catches_up_and_then_skips() {
    let elsewhere = World::new();
    elsewhere.add_full(1, 6);
    let cloud = FakeCloud::new();
    push_uploads(&elsewhere.conn, &elsewhere.workspaces_root, &cloud, &[SyncAction::Upload { doc_id: id(1), version: 6 }]);
    let world = World::new();
    world.add_full(1, 2);
    assert_eq!(compare(&world.conn, &cloud).expect("compare"), [pull_action(1, 6)]);

    let report = pull_downloads(&world.conn, &world.workspaces_root, &cloud, &[pull_action(1, 6)]);

    assert_eq!(report.pulled.len(), 1);
    assert_eq!(compare(&world.conn, &cloud).expect("compare"), [SyncAction::Skip { doc_id: id(1) }]);
    assert_eq!(world.inbox_files(), elsewhere.inbox_files());
    assert_eq!(get_doc(&world.conn, &id(1)).expect("query").map(|row| row.version), Some(6));
}
