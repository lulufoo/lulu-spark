use std::collections::HashMap;

use super::*;

fn map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
        .collect()
}

#[test]
fn rewrite_declared_images_rewrites_a_plain_image() {
    let declared = map(&[("a.png", ".attached/doc/a.png")]);
    let out = rewrite_declared_images("see ![x](a.png) now", &declared);
    assert_eq!(out.markdown, "see ![x](.attached/doc/a.png) now");
    assert!(out.unmatched.is_empty());
}

#[test]
fn rewrite_declared_images_keeps_a_title() {
    let declared = map(&[("a.png", ".attached/doc/a.png")]);
    let out = rewrite_declared_images(r#"![x](a.png "cap")"#, &declared);
    assert_eq!(out.markdown, r#"![x](.attached/doc/a.png "cap")"#);
}

#[test]
fn rewrite_declared_images_ignores_a_code_block_fake() {
    let declared = map(&[("a.png", ".attached/doc/a.png")]);
    let md = "```\n![x](a.png)\n```\n";
    let out = rewrite_declared_images(md, &declared);
    assert_eq!(out.markdown, md);
    assert!(out.unmatched.is_empty());
}

#[test]
fn rewrite_declared_images_ignores_html_img() {
    let declared = map(&[("a.png", ".attached/doc/a.png")]);
    let md = r#"<img src="a.png">"#;
    let out = rewrite_declared_images(md, &declared);
    assert_eq!(out.markdown, md);
    assert!(out.unmatched.is_empty());
}

#[test]
fn rewrite_declared_images_lists_an_unmatched_relative_ref() {
    let declared = map(&[("a.png", ".attached/doc/a.png")]);
    let out = rewrite_declared_images("![ok](a.png) ![miss](gone.png)", &declared);
    assert_eq!(out.markdown, "![ok](.attached/doc/a.png) ![miss](gone.png)");
    assert_eq!(out.unmatched, vec!["gone.png"]);
}

#[test]
fn rewrite_declared_images_matches_dot_slash_by_basename() {
    let declared = map(&[("a.png", ".attached/doc/a.png")]);
    let out = rewrite_declared_images("![x](./a.png)", &declared);
    assert_eq!(out.markdown, "![x](.attached/doc/a.png)");
}

#[test]
fn rewrite_declared_images_leaves_https_alone() {
    let declared = HashMap::new();
    let md = "![x](https://example.com/a.png)";
    let out = rewrite_declared_images(md, &declared);
    assert_eq!(out.markdown, md);
    assert!(out.unmatched.is_empty());
}
