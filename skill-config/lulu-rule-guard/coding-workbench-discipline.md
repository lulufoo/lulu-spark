# Workbench coding discipline (TS / JS)

> GitHub workflow discipline（Worktree / Commit / PR）：[docs/github-workflow.md](../../docs/github-workflow.md)

## Rules

1. **Tauri invoke (`*InvokeMap.js`)**: Payload keys camelCase (e.g. `filterType`); Rust snake_case. snake_case in payload is silently dropped.

2. **UI handlers**: Long work must be async; restore UI in `.finally()` (or `try/finally`).

3. **`@tauri-apps/*`**: No static import in `frontend/js/**` — Tauri only via `apiClient.js` or `window.__TAURI__` (`frontendTauriImportContract.test.js`).

## Git Authorization (❌ non-mechanizable — action triggers)

4. **Tool selection**: Use `gh` for GitHub ops (PR / issues / API); use `git` for local ops.

5. **Worktree workflow**: Follow create / clean-up steps in order; never skip or reorder.

6. **Conflict handling**: On any conflict, immediately run `git rebase --abort` or `git merge --abort`; surface full details to user — do not resolve independently.

7. **Commit / push — require confirmation**: Before any `git commit` or `git push`, show `git diff --stat` output and wait for explicit user confirmation ("yes" / "ok" / "confirm").

8. **PR — require confirmation**: Before `gh pr create`, present title, body, and `source → target` branch; wait for explicit user confirmation.

9. **Worktree naming**: branch `wt/<type>-<slug>`; directory `.cache/worktrees/<slug>/`.
   Types: `feat` / `fix` / `test` / `chore` / `docs` / `refactor`

10. **Commit message**: follow [Conventional Commits](https://www.conventionalcommits.org/) — `<type>(<scope>): <subject>`.
    Allowed types: `feat` / `fix` / `test` / `chore` / `docs` / `refactor` / `style` / `perf`
