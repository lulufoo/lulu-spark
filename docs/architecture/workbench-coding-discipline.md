# Lulu Spark Coding Discipline

## Rules

1. **Invoke maps (`frontend/src/host/*InvokeMap.ts`)**: HTTP / `host/api` bodies use snake_case (`common_path`). Invoke args use camelCase (`commonPath`). Rust command params are snake_case. snake_case in invoke args is silently dropped. Pages and commands do not import the maps — go through `host/api.ts` or the existing `apiClient` host.

2. **UI handlers**: Long work must be async; restore UI in `.finally()` (or `try/finally`).

3. **`@tauri-apps/*`**: No static import in `frontend/src/**`. Only `host/apiClient.ts` may dynamic-import (`frontendTauriImportContract.test.js`). Channel only via `createChannel`.

4. **Tauri command ACL (`src-tauri/permissions/*.toml`)**: New invoke commands must also go on the matching allowlist (`write-api` / `read-api` / `sync-api` / `search-api`).

5. **`docs/archive`**: Code and tests must not depend on it.
