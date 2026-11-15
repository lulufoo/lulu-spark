//! Bounded child-test execution with explicit disk receipts, without env overrides.

use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::Value;

const REQUEST_FILE: &str = "subprocess-request.json";

#[derive(Serialize, Deserialize)]
struct StageRequest {
    test_name: String,
    stage: String,
    parent_pid: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct StageReceipt {
    pub test_name: String,
    pub stage: String,
    pub parent_pid: u32,
    pub pid: u32,
    pub result: Value,
}

pub(crate) struct TestSubprocess {
    directory: tempfile::TempDir,
}

impl TestSubprocess {
    pub fn new() -> Self {
        Self { directory: tempfile::tempdir().expect("subprocess fixture directory") }
    }

    pub fn directory(&self) -> &Path {
        self.directory.path()
    }

    pub fn run_stage(&self, test_name: &str, stage: &str) -> StageReceipt {
        validate_stage(stage);
        let receipt_path = self.directory().join(format!("receipt-{stage}.json"));
        assert!(!receipt_path.exists(), "a stage cannot reuse a previous receipt");
        let request = StageRequest {
            test_name: test_name.into(), stage: stage.into(), parent_pid: std::process::id(),
        };
        std::fs::write(self.directory().join(REQUEST_FILE), serde_json::to_vec(&request).unwrap())
            .expect("write subprocess stage request");
        let mut child = Command::new(std::env::current_exe().expect("current test executable"))
            .args(["--ignored", "--exact", test_name, "--test-threads=1", "--nocapture"])
            .current_dir(self.directory())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn exact child test");
        let child_pid = child.id();
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            match child.try_wait() {
                Ok(Some(_)) => break,
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(Duration::from_millis(10));
                }
                outcome => {
                    let _ = child.kill();
                    let output = child.wait_with_output().expect("reap terminated child test");
                    panic!("child test {test_name} timed out or could not be polled: {outcome:?}\n{}\n{}",
                        String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
                }
            }
        }
        let output = child.wait_with_output().expect("collect child test output");
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(output.status.success(), "child test {test_name} failed:\n{stdout}\n{}",
            String::from_utf8_lossy(&output.stderr));
        assert!(stdout.contains("running 1 test\n"), "exactly one child test must run: {stdout}");
        assert!(stdout.contains("1 passed; 0 failed"), "the child test must pass: {stdout}");
        let receipt: StageReceipt = serde_json::from_slice(
            &std::fs::read(receipt_path).expect("the exact child test must produce its receipt"),
        ).expect("decode child test receipt");
        assert_eq!(receipt.test_name, test_name);
        assert_eq!(receipt.stage, stage);
        assert_eq!(receipt.parent_pid, std::process::id());
        assert_eq!(receipt.pid, child_pid, "receipt must come from the launched process");
        assert_ne!(receipt.pid, std::process::id());
        receipt
    }
}

pub(crate) fn run_stage_child(
    test_name: &str,
    stage: &str,
    operation: impl FnOnce(&Path) -> Value,
) {
    validate_stage(stage);
    let root = std::env::current_dir().expect("child fixture directory");
    let request: StageRequest = serde_json::from_slice(
        &std::fs::read(root.join(REQUEST_FILE)).expect("a child stage requires its parent fixture"),
    ).expect("decode subprocess stage request");
    assert_eq!(request.test_name, test_name);
    assert_eq!(request.stage, stage);
    assert_ne!(request.parent_pid, std::process::id());
    let receipt = StageReceipt {
        test_name: request.test_name,
        stage: request.stage,
        parent_pid: request.parent_pid,
        pid: std::process::id(),
        result: operation(&root),
    };
    std::fs::write(root.join(format!("receipt-{stage}.json")), serde_json::to_vec(&receipt).unwrap())
        .expect("write child test receipt");
}

fn validate_stage(stage: &str) {
    assert!(!stage.is_empty() && stage.bytes().all(|c| c.is_ascii_lowercase() || c == b'_'));
}
