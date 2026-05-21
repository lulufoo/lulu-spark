# Workbench coding discipline (TS / JS)

## Rules

1. **Tauri invoke (`*InvokeMap.js`)**: Payload keys camelCase (e.g. `filterType`); Rust snake_case. snake_case in payload is silently dropped.

2. **UI handlers**: Long work must be async; restore UI in `.finally()` (or `try/finally`).

3. **`@tauri-apps/*`**: No static import in `frontend/js/**` — Tauri only via `apiClient.js` or `window.__TAURI__` (`frontendTauriImportContract.test.js`).
