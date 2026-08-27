# Git Workflow Standard

Universal git action limits for AI-assisted development in this repo.

---

## Purpose and Scope

This document defines **what git operations AI may perform** — authorization, confirmation gates, forbidden actions, and conflict policy.

| Item | Detail |
|---|---|
| This doc | Git action limits (authorization, gates, forbidden ops, conflict policy) |
| Not in this doc | Workflow-specific procedures (worktree setup, per-task commits, delivery) → see the active workflow SKILL |
| GitHub URL content | See `docs/git/git-gh-operations.md` |
| Trigger | Read this doc before any git command |

---

## Authorization

**Questions, statements, and discussion are not authorization.**

The following require **explicit user request** before execution:

- `commit`, `push`, `merge`, `rebase`, `stash`
- Any `git config` change (never run unless the user explicitly requests it)

**Commits:** Create a commit only when the user explicitly asks. Do not create empty commits when there are no changes.

---

## Operation Tiers

### Tier A — Read-only (no confirm required)

`status` · `diff` · `diff --cached` · `log` · `show` · `branch` (list) · `worktree list`

### Tier B — Local write (confirm + user-authorized commit)

`add` · `commit` · `stash` / `stash pop`

### Tier C — Remote / integration (confirm + explicit authorization)

`push` · `pull` / `pull --rebase` · `merge`

---

## Confirmation Gates

| Operation | Show first | Wait for |
|---|---|---|
| `commit` | `git diff --cached --stat` (or equivalent staged diff) | Explicit `yes` / `ok` / `confirm` |
| `push` | `git log origin/<branch>..HEAD --oneline` | Explicit `yes` / `ok` / `confirm` |

Pass commit messages via HEREDOC to avoid shell escaping issues:

```bash
git commit -m "$(cat <<'EOF'
<type>(<scope>): <subject>

EOF
)"
```

---

## Forbidden — Destructive (AI must not execute)

AI must **never** run the following, even if the user asks in chat.

**STOP** — show the intended command and instruct the user to run it manually in their terminal:

- `reset --hard` / `reset --merge`
- `push --force` / `push -f`
- `branch -D`
- `clean -fd`
- `worktree remove` (when it may discard uncommitted work)

> `merge --abort` and `rebase --abort` are **allowed** under Conflict Policy — they exit a conflict safely and are not destructive ops.

---

## Forbidden and Restricted

| Rule | Behavior |
|---|---|
| `--no-verify` | Forbidden unless the user explicitly requests it |
| Force-push | Forbidden — no exception; see Forbidden — Destructive |
| Self-resolve conflicts | Forbidden — see Conflict Policy |
| Commit secrets | Do not commit `.env`, `credentials.json`, or similar; warn if the user asks to commit them |
| Change git config | Forbidden unless the user explicitly requests it |

---

## Conflict Policy

When `merge`, `rebase`, or `pull --rebase` hits a conflict:

1. Abort immediately (`merge --abort` or `rebase --abort`)
2. Report full conflict details to the user
3. **Never self-resolve**

---

## Out of Scope (STOP)

The following are not covered by this doc → **STOP** and tell the user to run manually or follow a workflow SKILL:

Interactive `rebase` · `cherry-pick` · `tag` · `commit --amend` · `bisect` · `submodule` · dedicated hotfix flows · other advanced git operations not listed above

> Out of scope does not mean permanently forbidden — AI must not run these automatically.

---

## Tool Split

| Scenario | Tool |
|---|---|
| Local git operations | `git` |
| GitHub: PRs, issues, checks, API | `gh` |
| Read files from GitHub URLs | Do not fetch via HTTP — read `docs/git/git-gh-operations.md` |

---

## Naming Conventions

| Item | Format |
|---|---|
| Commit message | `<type>(<scope>): <subject>` |
| Allowed types | `feat` `fix` `test` `chore` `docs` `refactor` `style` `perf` |
| Branch | Descriptive name; do not develop directly on protected branches (`main`) |

Branch and worktree path conventions for a specific workflow (e.g. `wt/<type>-<slug>`) are defined in that workflow's SKILL.

---

## Worktree Policy

**Code changes for feature work must happen in a worktree, not in the main checkout.**

Concrete commands and paths → see the active workflow SKILL (e.g. `lulu-dev-workflow/tech-code/SKILL.md` § Preparing).

---

## Failure Modes

| Scenario | Behavior |
|---|---|
| Pull / rebase conflict | Abort → STOP → report |
| Merge conflict | Abort → STOP → report |
| Push rejected by branch protection | STOP → suggest `gh pr create` |
| Dirty working tree blocks the operation | STOP → show `git status --short`; user decides stash / commit / revert |
| Worktree or branch already exists | STOP → report collision; do not delete or overwrite unilaterally |
| User requests a destructive op | STOP → show command; user runs manually |

---

## Non-Negotiable Summary

| Rule | Detail |
|---|---|
| Authorization | Commit / push / merge only on explicit user request |
| Confirmation | Show diff / log before commit and push; wait for confirm |
| Destructive ops | AI never executes; user runs manually |
| Conflicts | Abort and report; never self-resolve |
| No bypass | No `--no-verify`; never force-push |
| Secrets | Do not commit credential files |
| Tools | Local `git`; GitHub `gh` |
| Out of scope | STOP; instruct user to run manually |
| Worktree | Edit code in a worktree, not the main checkout |
