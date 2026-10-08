//! Context-window sizes by Host model name.

/// Local default when the Host model has no built-in row.
pub const DEFAULT_WINDOW_TOKENS: u64 = 255_000;

/// Tokens in the model's context window. Unknown models use the local default.
pub fn window_tokens(model: &str) -> u64 {
    match model {
        "glm-5.2" => 1_048_576,
        _ => DEFAULT_WINDOW_TOKENS,
    }
}

/// `round(tokens * 100 / window)`, half up. Values above 100 stay above 100.
pub fn round_percent(tokens: u64, window: u64) -> Option<i64> {
    if window == 0 {
        return None;
    }
    let tokens = i128::from(tokens);
    let window = i128::from(window);
    let rounded = (tokens * 100 + window / 2) / window;
    i64::try_from(rounded).ok()
}
