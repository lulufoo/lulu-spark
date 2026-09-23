//! Count tokens with the vendored GLM-5.2 tokenizer.

use std::sync::OnceLock;

use tokenizers::Tokenizer;

fn glm_tokenizer() -> &'static Tokenizer {
    static TOKENIZER: OnceLock<Tokenizer> = OnceLock::new();
    TOKENIZER.get_or_init(|| {
        Tokenizer::from_bytes(include_bytes!(
            "../../../assets/glm-5.2/tokenizer.json"
        ))
        .expect("vendored glm-5.2 tokenizer")
    })
}

/// Token count for text already rendered by the chat template.
/// Special tokens already in the text stay single tokens.
pub fn count_tokens(text: &str) -> Result<usize, String> {
    Ok(encode(text)?.len())
}

/// Assign each token to the span that contains its start byte.
/// Tokens outside every span count as `other`. The returned rows use the
/// display order and omit empty categories. Their sum is the token total.
pub fn count_spans(text: &str, spans: &[super::render::PromptSpan]) -> Result<Vec<(&'static str, usize)>, String> {
    let encoding = encode(text)?;
    let mut counts = [0usize; CATEGORY_ORDER.len()];
    for (start, _) in encoding.get_offsets() {
        let id = spans
            .iter()
            .find(|span| *start >= span.start && *start < span.end)
            .map(|span| span.id)
            .unwrap_or("other");
        let index = CATEGORY_ORDER
            .iter()
            .position(|name| *name == id)
            .unwrap_or(CATEGORY_ORDER.len() - 1);
        counts[index] += 1;
    }
    Ok(CATEGORY_ORDER
        .iter()
        .zip(counts)
        .filter(|(_, count)| *count > 0)
        .map(|(id, count)| (*id, count))
        .collect())
}

const CATEGORY_ORDER: [&str; 5] = ["system_prompt", "tools", "mcp", "conversation", "other"];

fn encode(text: &str) -> Result<tokenizers::Encoding, String> {
    glm_tokenizer()
        .encode(text, false)
        .map_err(|error| error.to_string())
}
