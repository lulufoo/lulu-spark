# Workbench testing discipline (Rust / src-tauri)

## Rules

1. **Unit test location**: Put tests in `src/unit-tests/{domain}/{same-name}.rs`. In the source file, keep only `#[cfg(test)]`, `#[path = "../unit-tests/..."]`, and `mod tests;` — never an inline `mod tests { }`.

2. **`read_api_acl_contract`**: No production source file. Mount from `config/mod.rs` via `#[path = "../unit-tests/config/read_api_acl_contract.rs"]`.

3. **`lib.rs` named modules**: `wait_for_port_tests`, `ping_tests`, and `spawn_decision_tests` use `#[path = "unit-tests/lib/<name>.rs"]` (relative to `src/`, no `../`). Register `test_support` once: `#[cfg(test)] mod test_support;`.

4. **Integration tests**: Use `src-tauri/tests/` when added later; not part of the current lib-unit layout.

5. **Run tests**: From `src-tauri/`, run `cargo nextest run --lib` (install with `cargo install cargo-nextest --locked`). Config: `.config/nextest.toml`.

6. **Fixtures and isolation**: Do not add `ENV_LOCK` / test mutexes — nextest runs each case in its own process. Prefer `crate::test_support::{with_test_config_dir, with_corpus}` over copy-pasted helpers. `settings` tests may use `EnvGuard` (RAII cleanup, no mutex).

7. **Before commit**: `cargo nextest run --lib` must pass. `grep -rn '^[[:space:]]*mod tests[[:space:]]*{' src-tauri/src/` must print nothing.
