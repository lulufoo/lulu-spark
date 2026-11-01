use std::sync::Mutex;
use std::time::Duration;

use serde_json::{json, Value};

use super::*;
use crate::services::app_log::Level;
use crate::services::doc_sync::tests::{id, Call, FakeCloud};

/// The lines a `Logged` cloud handed out, in the order it did.
struct Lines(Mutex<Vec<Line>>);

impl Lines {
    fn new() -> Self {
        Self(Mutex::new(Vec::new()))
    }

    fn sink(&self) -> impl Fn(Line) + '_ {
        |line| self.0.lock().expect("lines").push(line)
    }

    fn all(&self) -> Vec<Line> {
        self.0.lock().expect("lines").clone()
    }
}

/// The line's fields without the time, and the time. The time of a call is not a number a test can
/// know, only that it is there.
fn without_ms(line: &Line) -> (Value, Option<u64>) {
    let mut params = line.params.clone();
    let ms = params.as_object_mut().and_then(|p| p.remove("ms")).and_then(|ms| ms.as_u64());
    (params, ms)
}

#[test]
fn what_goes_through_is_logged_with_what_it_was() {
    type Call = fn(&Logged);
    let cases: Vec<(&str, Call, &str, Value)> = vec![
        ("a list", |c| drop(c.list_documents()), "sync.listed", json!({ "cloud_docs": 1 })),
        (
            "an upload",
            |c| drop(c.upload(&id(2), b"abc", 4)),
            "sync.doc_uploaded",
            json!({ "doc_id": id(2), "version": 4, "bytes": 3 }),
        ),
        (
            "a read",
            |c| drop(c.read(&id(1))),
            "sync.doc_downloaded",
            json!({ "doc_id": id(1), "bytes": 7 }),
        ),
    ];
    for (name, call, event, fields) in cases {
        let fake = FakeCloud::new().with_package(&id(1), 3, b"package".to_vec());
        let lines = Lines::new();
        let sink = lines.sink();

        call(&Logged::new(&fake, &sink));

        let written = lines.all();
        assert_eq!(written.len(), 1, "{name}: one line");
        assert_eq!((written[0].level, written[0].event), (Level::Info, event), "{name}");
        let (params, ms) = without_ms(&written[0]);
        assert_eq!(params, fields, "{name}: nothing but these fields");
        assert!(ms.is_some(), "{name}: the time is a number of milliseconds: {:?}", written[0].params);
    }
}

#[test]
fn what_went_wrong_is_returned_as_it_was_and_not_logged() {
    let gone = || CloudSyncError::Network("down".into());
    let fake = FakeCloud::new()
        .failing_list(gone())
        .failing_upload(&id(1), gone())
        .failing_read(&id(1), CloudSyncError::Unauthorized);
    let lines = Lines::new();
    let sink = lines.sink();
    let cloud = Logged::new(&fake, &sink);

    assert_eq!(cloud.list_documents(), Err(gone()));
    assert_eq!(cloud.upload(&id(1), b"x", 1), Err(gone()));
    assert_eq!(cloud.read(&id(1)), Err(CloudSyncError::Unauthorized));

    assert_eq!(lines.all(), Vec::<Line>::new());
}

#[test]
fn every_call_reaches_the_cloud_as_it_was_made_and_a_delete_is_not_logged() {
    let fake = FakeCloud::new().with_package(&id(1), 3, b"package".to_vec());
    let lines = Lines::new();
    let sink = lines.sink();
    let cloud = Logged::new(&fake, &sink);

    cloud.list_documents().expect("list");
    cloud.upload(&id(2), b"new", 4).expect("upload");
    cloud.read(&id(1)).expect("read");
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
    let events: Vec<&str> = lines.all().iter().map(|l| l.event).collect();
    assert_eq!(events, ["sync.listed", "sync.doc_uploaded", "sync.doc_downloaded"]);
}

/// A cloud that takes its time over every call.
struct Slow(FakeCloud);

const DELAY: Duration = Duration::from_millis(30);

impl CloudDocs for Slow {
    fn list_documents(&self) -> Result<Vec<CloudDoc>, CloudSyncError> {
        std::thread::sleep(DELAY);
        self.0.list_documents()
    }

    fn upload(&self, document_id: &str, package: &[u8], version: i64) -> Result<(), CloudSyncError> {
        std::thread::sleep(DELAY);
        self.0.upload(document_id, package, version)
    }

    fn read(&self, document_id: &str) -> Result<Vec<u8>, CloudSyncError> {
        std::thread::sleep(DELAY);
        self.0.read(document_id)
    }

    fn delete(&self, document_id: &str) -> Result<(), CloudSyncError> {
        self.0.delete(document_id)
    }
}

#[test]
fn the_time_of_a_line_is_the_time_the_cloud_took() {
    let slow = Slow(FakeCloud::new().with_package(&id(1), 3, b"package".to_vec()));
    let lines = Lines::new();
    let sink = lines.sink();
    let cloud = Logged::new(&slow, &sink);

    cloud.list_documents().expect("list");
    cloud.upload(&id(2), b"new", 4).expect("upload");
    cloud.read(&id(1)).expect("read");

    for line in lines.all() {
        let (_, ms) = without_ms(&line);
        assert!(ms.expect("ms") >= DELAY.as_millis() as u64, "{}: {:?}", line.event, line.params);
    }
}
