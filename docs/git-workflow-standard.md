# Git Workflow Standard

---

## State Variables

| Variable | Set in | Read in | Values |
|---|---|---|---|
| `DID_STASH` | P1 | P3 | `true` / `false` |
| `PRE_MERGE` | P5 | E5 | git SHA |

---

## Flow

Normal path: **P1 → P2 → P3 → P4 → P5 → P6 → P7 → P8**

Exception paths:
- P6 fail → **E4** → P6
- P3 stash conflict → **E1** → P4
- Any point before P7 → **E5** (Abandon)

---

## Naming Convention

| Item | Format |
|---|---|
| Branch | `wt/<type>-<slug>` |
| Worktree directory | `.cache/worktrees/<slug>/` |
| Commit message | `<type>(<scope>): <subject>` |
| Allowed types | `feat` `fix` `test` `chore` `docs` `refactor` `style` `perf` |

---

## P1 — Pre-check

1. `git status`
2. `git worktree list`
3. IF dirty: `git stash` → `DID_STASH=true`  
   ELSE: `DID_STASH=false`
4. IF `.cache/worktrees/<slug>/` already exists OR branch `wt/<type>-<slug>` already exists:  
   **STOP** — report collision to user; do not self-resolve

---

## P2 — Sync

1. `git pull --rebase`
2. IF conflict: `git rebase --abort` → **STOP** — report conflict details to user

---

## P3 — Create Worktree

1. `git worktree add .cache/worktrees/<slug> -b wt/<type>-<slug>`
2. `cd .cache/worktrees/<slug>/`
3. IF `DID_STASH=true`:
   - `git stash pop`
   - IF conflict → **E1**

> Worktrees share the stash stack. `git stash pop` must run inside the worktree directory.

---

## P4 — Stage & Commit `[CONFIRM]`

Working dir: `.cache/worktrees/<slug>/`

1. `git add <files>`
2. `git diff --cached --stat` — show output to user
3. **[CONFIRM]** — wait for explicit "yes" / "ok" / "confirm"
4. `git commit -m "<type>(<scope>): <subject>"`

---

## P5 — Merge

1. `cd <original-dir>` (back to main checkout)
2. `PRE_MERGE=$(git rev-parse HEAD)`
3. `git merge --no-ff wt/<type>-<slug>`
4. IF conflict: `git merge --abort` → **STOP** — report full conflict details to user

> `--no-ff` always creates a merge commit. `PRE_MERGE` is required by E5.

---

## P6 — Test

Run tests. No git commands.  
IF fail → **E4**

---

## P7 — Push `[CONFIRM]`

1. `git log origin/<branch>..HEAD --oneline` — show output to user
2. **[CONFIRM]** — wait for explicit "yes" / "ok" / "confirm"
3. `git push`
4. IF rejected (branch protection): **STOP** — report; direct push not allowed; use `gh pr create`

---

## P8 — Cleanup

1. `git worktree remove .cache/worktrees/<slug>`
2. `git branch -d wt/<type>-<slug>`

---

## E1 — Stash Pop Conflict

Triggered from P3.

1. Show conflict files to user
2. **WAIT** — user resolves manually in `.cache/worktrees/<slug>/`
3. `git add <resolved-files>`
4. `git stash drop`
5. → return to P4

---

## E4 — Test Fail Fix `[CONFIRM]`

Triggered from P6. Working dir = original dir (current branch). Do **not** go back to worktree.

1. `git add <files>`
2. `git diff --cached --stat` — show to user
3. **[CONFIRM]**
4. `git commit -m "fix(<scope>): <subject>"`
5. → return to P6

---

## E5 — Abandon

Triggered any time before P7. Requires `PRE_MERGE` from P5.

1. `git log --oneline -3` — show to user
2. **[CONFIRM]** reset to `$PRE_MERGE`
3. `git reset --hard $PRE_MERGE`
4. `git worktree remove .cache/worktrees/<slug>`
5. `git branch -D wt/<type>-<slug>`

> `-D` (force delete) — branch is no longer reachable after reset.

---

## Edge Case Reference

| # | Trigger | Handled In |
|---|---|---|
| A | Working tree dirty before pull | P1 + P3 |
| B | `git stash pop` conflict | E1 |
| C | `git pull --rebase` conflict | P2 |
| D | Worktree directory already exists | P1 |
| E | Branch `wt/type-slug` already exists | P1 |
| F | Test fail after merge | E4 |
| G | Abandon after merge, before push | E5 |
| H | Push blocked by branch protection | P7 |

---

## Non-negotiable Rules

| Rule | Detail |
|---|---|
| Tool selection | `gh` for GitHub ops (PR / API); `git` for local ops |
| Confirmation gate | Show diff before every `commit` and `push`; wait for explicit "yes" / "ok" / "confirm" |
| Conflict handling | Immediately abort; report full details to user; never self-resolve |
| No bypass | Never use `--no-verify` or force-push without explicit authorization |
