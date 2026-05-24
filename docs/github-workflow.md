# GitHub Workflow

## Tool Convention

| Tool | Purpose |
|---|---|
| `gh` | Create/view PRs, manage issues, GitHub API calls |
| `git` | Local operations (add / commit / stash / rebase, etc.) |

---

## Core Principle: All Changes Must Go Through a Worktree

**Direct commit or push to the current working branch is prohibited.**

All code changes (feature development, bug fixes, tests, chores, etc.) must be made inside a worktree. The worktree must be removed when the work is done.

---

## Worktree Directory and Naming

### Directory

All worktrees are placed under `.cache/worktrees/<slug>/`. The `.cache/` directory is already in `.gitignore`.

### Naming Convention

Branch format: `wt/<type>-<slug>`. The directory name is the `<slug>` portion. Choose a type by task category:

| Task Type | Branch Prefix | Example |
|---|---|---|
| Feature development | `wt/feat-` | `wt/feat-add-search-filter` |
| Bug fix | `wt/fix-` | `wt/fix-comment-reorder` |
| Testing | `wt/test-` | `wt/test-api-contract` |
| Chore / config | `wt/chore-` | `wt/chore-update-deps` |
| Documentation | `wt/docs-` | `wt/docs-github-workflow` |
| Refactoring | `wt/refactor-` | `wt/refactor-state-module` |

---

## Worktree Workflow

### Create

```bash
# 1. Stash uncommitted changes (include slug for clarity)
git stash push -m "wip: before-worktree-<slug>"

# 2. Sync with remote (if rebase fails: abort and surface the error immediately)
git pull --rebase

# 3. Create the worktree
git worktree add .cache/worktrees/<slug> -b wt/<type>-<slug>

# 4. Restore uncommitted changes to the original branch (they do not carry into the worktree)
git stash pop
```

> The worktree and the original branch are fully independent. Uncommitted changes from the original branch do not enter the worktree.

### Clean Up (after PR is merged)

```bash
git worktree remove .cache/worktrees/<slug>
git branch -d wt/<type>-<slug>
```

---

## Conflict Handling

When a conflict is detected (during `pull --rebase` or a merge/rebase inside the worktree):

1. Immediately run abort — **do not leave the repository in a mid-conflict state**:
   - `git rebase --abort`
   - `git merge --abort`
2. Surface the full conflict details to the user. AI must not resolve conflicts independently.

---

## Commit Message Convention

Follow [Conventional Commits](https://www.conventionalcommits.org/):

```
<type>(<scope>): <subject>

[optional body]
```

**Allowed types**: `feat` / `fix` / `test` / `chore` / `docs` / `refactor` / `style` / `perf`

**Examples**:

```
feat(search): add filter by tag
fix(comment): correct reorder index calculation
test(api): add contract test for write api
```

---

## Commit / Push Authorization

**AI must not commit or push inside a worktree without user authorization.**

Before committing:

1. Show `git diff --stat` (file-level change summary) so the user can review
2. Wait for explicit confirmation from the user in the conversation ("yes" / "ok" / "confirm", etc.)

---

## PR Authorization

**AI must not create a PR without user authorization.**

Before running `gh pr create`, present:

- PR title
- PR body (summary of changes)
- source branch → target branch

Wait for explicit user confirmation or a direct request before proceeding.
