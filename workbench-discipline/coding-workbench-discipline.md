# Workbench coding discipline (TS / JS)

## Rules

1. **Tauri invoke (`*InvokeMap.js`)**: Use camelCase for multi-word keys in the `invoke` payload (e.g. `filterType`); Rust stays snake_case. Do not pass snake_case in the payload or the argument is silently dropped.

2. **UI event handler + blocking ops**: Blocking or long-running operations must be async — never run them synchronously in a UI handler as they freeze the main thread. When going async, always restore UI state in `.finally()` (or `try/finally`).
