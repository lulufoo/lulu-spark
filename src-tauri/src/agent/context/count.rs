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
    let encoding = glm_tokenizer()
        .encode(text, false)
        .map_err(|error| error.to_string())?;
    Ok(encoding.len())
}
