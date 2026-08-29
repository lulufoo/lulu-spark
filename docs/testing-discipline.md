# Workbench testing discipline (Rust / src-tauri)

## Rules

1. **Unit test location**: Put tests in `src/unit-tests/{domain}/{same-name}.rs`. In the source file, keep only `#[cfg(test)]`, `#[path = "../unit-tests/..."]`, and `mod tests;` — never an inline `mod tests { }`.

2. **`read_api_acl_contract`**: No production source file. Mount from `config/mod.rs` via `#[path = "../unit-tests/config/read_api_acl_contract.rs"]`.

3. **`lib.rs` named modules**: `wait_for_port_tests`, `ping_tests`, and `spawn_decision_tests` use `#[path = "unit-tests/lib/<name>.rs"]` (relative to `src/`, no `../`). Register `test_support` once: `#[cfg(test)] mod test_support;`.

4. **Integration tests**: Use `src-tauri/tests/` when added later; not part of the current lib-unit layout.

5. **Run tests**: From `src-tauri/`, run `cargo nextest run --lib` (install with `cargo install cargo-nextest --locked`). Config: `.config/nextest.toml`.

6. **Fixtures and isolation**: ✅ Verified (`src-tauri/src/test_support.rs`): Do not mutate process environment variables directly or add ad-hoc environment locks, test mutexes, `EnvGuard`, or equivalent bypasses. Environment-dependent tests must use `crate::test_support::TestConfigEnv`; nextest provides process-level parallelism, while `TestConfigEnv` serializes environment lifecycles within one process. ✅ Verified (`src-tauri/.config/nextest.toml`): the fixed formal-port close-gate tests run in `mcp-formal-ports` with `max-threads = 1`; they still require `8765` and `9876` to be free. Prefer shared fixtures such as `TestSandbox` and `with_sandbox_notes` over copy-pasted helpers.

7. **Before commit**: `cargo nextest run --lib` must pass. `grep -rn '^[[:space:]]*mod tests[[:space:]]*{' src-tauri/src/` must print nothing.
