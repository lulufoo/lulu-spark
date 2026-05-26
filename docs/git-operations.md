# Git Authorization (❌ non-mechanizable — action triggers)

1. **Tool selection**: Use `gh` for GitHub ops (PR / issues / API); use `git` for local ops.

2. **Worktree workflow**: Follow create / clean-up steps in order; never skip or reorder.

3. **Conflict handling**: On any conflict, immediately run `git rebase --abort` or `git merge --abort`; surface full details to user — do not resolve independently.

4. **Commit / push — require confirmation**: Before any `git commit` or `git push`, show `git diff --stat` output and wait for explicit user confirmation ("yes" / "ok" / "confirm").

5. **PR — require confirmation**: Before `gh pr create`, present title, body, and `source → target` branch; wait for explicit user confirmation.

6. **Worktree naming**: branch `wt/<type>-<slug>`; directory `.cache/worktrees/<slug>/`.
   Types: `feat` / `fix` / `test` / `chore` / `docs` / `refactor`

7. **Worktree create**:
```bash
git worktree add .cache/worktrees/<slug> -b wt/<type>-<slug>
```

8. **Commit message**: follow [Conventional Commits](https://www.conventionalcommits.org/) — `<type>(<scope>): <subject>`.
    Allowed types: `feat` / `fix` / `test` / `chore` / `docs` / `refactor` / `style` / `perf`

9. **Worktree clean-up (after PR is merged)**:
```bash
git pull --rebase
git worktree remove .cache/worktrees/<slug>
git branch -d wt/<type>-<slug>
```
