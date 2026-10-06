use std::path::Path;

use super::{
    github_repo_url_from_remote_url, github_user_home_from_remote_url, spark_github_blob_base,
};

#[test]
fn github_remote_helpers_remain_unchanged() {
    assert_eq!(
        github_user_home_from_remote_url("git@github.com:lulufoo/project.git"),
        Some("https://github.com/lulufoo".into())
    );
    assert_eq!(
        spark_github_blob_base(
            "https://github.com/lulufoo",
            Path::new("/Users/me/Code/lulu-workbench-knowledge"),
        ),
        "https://github.com/lulufoo/lulu-workbench-knowledge/blob/main"
    );
    assert_eq!(
        github_repo_url_from_remote_url("git@github.com:lulufoo/lulu-workbench-knowledge.git"),
        Some("https://github.com/lulufoo/lulu-workbench-knowledge".into())
    );
    assert_eq!(
        github_repo_url_from_remote_url("https://github.com/lulufoo/notes.git"),
        Some("https://github.com/lulufoo/notes".into())
    );
}
