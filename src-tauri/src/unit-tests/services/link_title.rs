use super::*;

#[test]
fn fallback_title_strips_md() {
    assert_eq!(
        fallback_title("https://github.com/o/r/blob/main/docs/foo.md"),
        "foo"
    );
}

#[test]
fn parse_title_tag_extracts() {
    let html = "<html><head><title>Hello World</title></head></html>";
    assert_eq!(parse_title_tag(html).as_deref(), Some("Hello World"));
}
