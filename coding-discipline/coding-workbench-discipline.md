# Workbench coding discipline (TS / JS)

## Rules

1. **Tauri invoke (`*InvokeMap.js`)**: Use camelCase for multi-word keys in the `invoke` payload (e.g. `filterType`); Rust stays snake_case. Do not pass snake_case in the payload or the argument is silently dropped.
