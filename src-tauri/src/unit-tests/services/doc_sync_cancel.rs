use super::*;
use crate::services::doc_sync::tests::{id, Call, FakeCloud};
use crate::services::doc_sync::SyncGate;

#[test]
fn until_the_user_cancels_every_call_goes_through() {
    let gate = SyncGate::new();
    let run = gate.try_begin().expect("run");
    let fake = FakeCloud::new().with_package(&id(1), 3, b"package".to_vec());
    let cloud = Cancellable::new(&fake, &run);

    assert_eq!(cloud.list_documents().expect("list").len(), 1);
    cloud.upload(&id(2), b"new", 4).expect("upload");
    assert_eq!(cloud.read(&id(1)).expect("read"), b"package");
    cloud.delete(&id(1)).expect("delete");

    assert_eq!(
        fake.calls(),
        [
            Call::List,
            Call::Upload { doc_id: id(2), version: 4, bytes: 3 },
            Call::Read { doc_id: id(1) },
            Call::Delete { doc_id: id(1) },
        ]
    );
}

#[test]
fn once_the_user_cancels_the_transfers_are_refused_and_never_reach_the_cloud() {
    let gate = SyncGate::new();
    let run = gate.try_begin().expect("run");
    let fake = FakeCloud::new().with_package(&id(1), 3, b"package".to_vec());
    let cloud = Cancellable::new(&fake, &run);
    gate.cancel();

    assert_eq!(cloud.upload(&id(2), b"new", 4), Err(CloudSyncError::Cancelled));
    assert_eq!(cloud.read(&id(1)), Err(CloudSyncError::Cancelled));

    assert_eq!(fake.calls(), Vec::<Call>::new(), "a refused transfer is not sent");
    assert_eq!(fake.stored(&id(2)), None);
}

#[test]
fn the_list_and_a_delete_are_not_held_back_by_a_cancel() {
    let gate = SyncGate::new();
    let run = gate.try_begin().expect("run");
    let fake = FakeCloud::new().with(&id(1), 3);
    let cloud = Cancellable::new(&fake, &run);
    gate.cancel();

    assert_eq!(cloud.list_documents().expect("list").len(), 1, "the plan can still be made");
    cloud.delete(&id(1)).expect("delete is not a part of a run");

    assert_eq!(fake.calls(), [Call::List, Call::Delete { doc_id: id(1) }]);
}

#[test]
fn the_error_of_the_cloud_is_passed_on_unchanged_when_nobody_cancelled() {
    let gate = SyncGate::new();
    let run = gate.try_begin().expect("run");
    let fake = FakeCloud::new().failing_upload(&id(1), CloudSyncError::QuotaExceeded { limit_bytes: 9, used_bytes: 9 });
    let cloud = Cancellable::new(&fake, &run);

    assert_eq!(cloud.upload(&id(1), b"x", 1), Err(CloudSyncError::QuotaExceeded { limit_bytes: 9, used_bytes: 9 }));
    assert_eq!(cloud.read(&id(7)), Err(CloudSyncError::ObjectNotFound));
}
