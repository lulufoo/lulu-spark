//! Application-side CJK bigrams for the unicode61 shadow table.

pub fn is_cjk(c: char) -> bool {
    matches!(
        c,
        '\u{4E00}'..='\u{9FFF}'
            | '\u{3400}'..='\u{4DBF}'
            | '\u{F900}'..='\u{FAFF}'
            | '\u{20000}'..='\u{2A6DF}'
    )
}

/// CJK runs become overlapping 2-grams; non-CJK runs stay as words.
pub fn cjk_bigram_text(text: &str) -> String {
    let mut out = Vec::new();
    let mut cjk_run = String::new();
    let mut other = String::new();

    let flush_cjk = |run: &mut String, out: &mut Vec<String>| {
        if run.is_empty() {
            return;
        }
        let chars: Vec<char> = run.chars().collect();
        if chars.len() == 1 {
            out.push(chars[0].to_string());
        } else {
            for w in chars.windows(2) {
                out.push(format!("{}{}", w[0], w[1]));
            }
        }
        run.clear();
    };
    let flush_other = |other: &mut String, out: &mut Vec<String>| {
        let t = other.trim();
        if !t.is_empty() {
            out.push(t.to_string());
        }
        other.clear();
    };

    for c in text.chars() {
        if is_cjk(c) {
            flush_other(&mut other, &mut out);
            cjk_run.push(c);
        } else if c.is_whitespace() {
            flush_cjk(&mut cjk_run, &mut out);
            flush_other(&mut other, &mut out);
        } else {
            flush_cjk(&mut cjk_run, &mut out);
            other.push(c);
        }
    }
    flush_cjk(&mut cjk_run, &mut out);
    flush_other(&mut other, &mut out);
    out.join(" ")
}
