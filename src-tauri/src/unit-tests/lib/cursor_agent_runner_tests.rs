use std::path::{Path, PathBuf};

use super::*;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..")
}

#[test]
fn cursor_agent_runner_launch_path_points_at_package_entry() {
    let root = repo_root();
    let path = cursor_agent_runner_launch_path(&root);
    assert!(
        path.ends_with(Path::new("packages/cursor-agent-runner/dist/index.js")),
        "unexpected launch path: {}",
        path.display()
    );
}

#[test]
fn cursor_agent_runner_launch_path_exists_in_repo_checkout() {
    let root = repo_root();
    let path = cursor_agent_runner_launch_path(&root);
    // Built artifact may be absent before `npm run build`; source package must exist.
    let pkg = root.join("packages/cursor-agent-runner/package.json");
    assert!(
        pkg.is_file(),
        "cursor-agent-runner package.json missing at {}",
        pkg.display()
    );
    let src_entry = root.join("packages/cursor-agent-runner/src/index.ts");
    assert!(
        src_entry.is_file(),
        "cursor-agent-runner src/index.ts missing at {}",
        src_entry.display()
    );
    assert_eq!(path, root.join("packages/cursor-agent-runner/dist/index.js"));
}
