//! Parse and validate archive documents (`theme-summary` Core Output Shape).

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedDocument {
    pub common_path: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseError {
    MissingTitle,
    MissingCreatedAt,
    MissingDigestNav,
    InvalidCommonPath(String),
    EmptyBody,
}

impl ParseError {
    pub fn message(&self) -> String {
        match self {
            ParseError::MissingTitle => "missing title (# heading)".into(),
            ParseError::MissingCreatedAt => "missing > 创建时间： line".into(),
            ParseError::MissingDigestNav => "missing [digest](...) navigation link".into(),
            ParseError::InvalidCommonPath(p) => format!("invalid common_path: {p}"),
            ParseError::EmptyBody => "empty body after ---".into(),
        }
    }
}

pub fn parse_archive_document(document: &str) -> Result<ParsedDocument, ParseError> {
    let trimmed = document.trim();
    if !trimmed.starts_with("# ") {
        return Err(ParseError::MissingTitle);
    }
    if !trimmed.lines().any(|l| l.contains("> 创建时间：")) {
        return Err(ParseError::MissingCreatedAt);
    }

    let common_path = extract_common_path_from_digest_link(trimmed)
        .ok_or(ParseError::MissingDigestNav)?;
    if !is_valid_common_path(&common_path) {
        return Err(ParseError::InvalidCommonPath(common_path));
    }

    let created_at = ts_from_common_path(&common_path)
        .ok_or_else(|| ParseError::InvalidCommonPath(common_path.clone()))?;

    let body = body_after_separator(trimmed);
    if body.trim().is_empty() {
        return Err(ParseError::EmptyBody);
    }

    Ok(ParsedDocument {
        common_path,
        created_at,
    })
}

fn body_after_separator(document: &str) -> &str {
    if let Some(idx) = document.find("\n---\n") {
        return &document[idx + 5..];
    }
    if let Some(idx) = document.find("\n---\r\n") {
        return &document[idx + 6..];
    }
    ""
}

fn extract_common_path_from_digest_link(document: &str) -> Option<String> {
    let marker = "[digest](";
    let start = document.find(marker)? + marker.len();
    let rest = &document[start..];
    let end = rest.find(')')?;
    let target = rest[..end].trim();
    normalize_common_path_from_digest_target(target)
}

fn normalize_common_path_from_digest_target(target: &str) -> Option<String> {
    let path = target.split('#').next()?.trim();
    let path = path.trim_matches(|c| c == '<' || c == '>' || c == '`');
    let path = path.replace('\\', "/");

    let common = if let Some(idx) = path.find("digest/") {
        path[idx + "digest/".len()..].to_string()
    } else if path.starts_with('/') {
        return None;
    } else {
        path
    };

    let common = common.trim_start_matches("./").to_string();
    if common.is_empty() || common.contains("..") {
        return None;
    }
    Some(common)
}

pub fn expected_lang_common_path(primary_common_path: &str, lang: &str) -> Option<String> {
    if !primary_common_path.ends_with(".md") {
        return None;
    }
    let stem = &primary_common_path[..primary_common_path.len() - 3];
    Some(format!("{stem}-{lang}.md"))
}

pub fn expected_zh_common_path(primary_common_path: &str) -> Option<String> {
    expected_lang_common_path(primary_common_path, "zh")
}

fn is_valid_common_path(common_path: &str) -> bool {
    let parts: Vec<&str> = common_path.split('/').collect();
    if parts.len() != 3 {
        return false;
    }
    let filename = parts[2];
    if !filename.ends_with(".md") {
        return false;
    }
    let stem = &filename[..filename.len() - 3];
    let Some((ts, slug)) = stem.split_once('-') else {
        return false;
    };
    ts.len() == 12
        && ts.chars().all(|c| c.is_ascii_digit())
        && !slug.is_empty()
        && slug
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

pub fn ts_from_common_path(common_path: &str) -> Option<String> {
    let filename = common_path.rsplit('/').next()?;
    if filename.len() < 13 {
        return None;
    }
    let ts = &filename[..12];
    if ts.chars().all(|c| c.is_ascii_digit()) {
        Some(ts.to_string())
    } else {
        None
    }
}

pub fn is_valid_entry_id(id: &str) -> bool {
    id.len() == 32
        && id
            .chars()
            .all(|c| c.is_ascii_hexdigit() && !c.is_uppercase())
}

#[cfg(test)]
#[path = "../unit-tests/services/archive_parse.rs"]
mod tests;
