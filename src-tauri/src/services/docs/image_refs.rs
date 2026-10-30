//! Scan Markdown image destinations (`![]()` only) and rewrite declared attachments.
//! HTML `<img>` is left as text. No IO.

use std::collections::HashMap;
use std::ops::Range;

use pulldown_cmark::{Event, LinkType, Options, Parser, Tag};

pub(crate) struct DeclaredImageRewrite {
    pub markdown: String,
    pub unmatched: Vec<String>,
}

/// Rewrite inline `![]()` destinations whose basename is in `new_href_by_basename`.
/// Relative destinations with no declared basename are listed and left as-is.
pub(crate) fn rewrite_declared_images(
    markdown: &str,
    new_href_by_basename: &HashMap<String, String>,
) -> DeclaredImageRewrite {
    let mut replacements: Vec<(Range<usize>, String)> = Vec::new();
    let mut unmatched = Vec::new();
    let parser = Parser::new_ext(markdown, Options::empty());
    for (event, range) in parser.into_offset_iter() {
        let Event::Start(Tag::Image {
            link_type: LinkType::Inline,
            dest_url,
            ..
        }) = event
        else {
            continue;
        };
        let dest = dest_url.as_ref();
        let Some(dest_range) = dest_range_in(markdown, range, dest) else {
            continue;
        };
        let basename = href_basename(dest);
        if let Some(new_href) = new_href_by_basename.get(basename) {
            if new_href != dest {
                replacements.push((dest_range, new_href.clone()));
            }
            continue;
        }
        if is_relative_href(dest) && !unmatched.iter().any(|s| s == dest) {
            unmatched.push(dest.to_string());
        }
    }
    DeclaredImageRewrite {
        markdown: apply_replacements(markdown, replacements),
        unmatched,
    }
}

fn dest_range_in(markdown: &str, img_range: Range<usize>, dest: &str) -> Option<Range<usize>> {
    if dest.is_empty() {
        return None;
    }
    let slice = markdown.get(img_range.clone())?;
    let after_paren = slice.find("](").map(|i| i + 2).unwrap_or(0);
    let haystack = slice.get(after_paren..)?;
    let rel = haystack.find(dest)?;
    let start = img_range.start + after_paren + rel;
    Some(start..start + dest.len())
}

fn apply_replacements(markdown: &str, mut replacements: Vec<(Range<usize>, String)>) -> String {
    replacements.sort_by_key(|(range, _)| range.start);
    replacements.reverse();
    let mut out = markdown.to_string();
    for (range, text) in replacements {
        out.replace_range(range, &text);
    }
    out
}

fn href_basename(href: &str) -> &str {
    href.rsplit(['/', '\\']).next().unwrap_or(href)
}

fn is_relative_href(href: &str) -> bool {
    let href = href.trim();
    if href.is_empty() || href.starts_with('#') || href.starts_with('/') {
        return false;
    }
    let lower = href.to_ascii_lowercase();
    !(lower.starts_with("https://")
        || lower.starts_with("http://")
        || lower.starts_with("data:")
        || lower.starts_with("blob:"))
}

#[cfg(test)]
#[path = "../../unit-tests/services/docs_image_refs.rs"]
mod tests;
