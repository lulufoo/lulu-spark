# Refactor 3.1 — 索引机制重构

> 基于 3.0 已完成的 Meilisearch 集成，本次重构解决三个问题：
> 1. UI 入口杂乱（stale banner 逻辑复杂）
> 2. git clone 静默失败导致 repo 缺失
> 3. 每次触发全量重建性能差（~45s）

---

## TL;DR

四个 Phase 独立可执行，按顺序实施。

| Phase | 标题 | 主要文件 | 收益 |
|-------|------|---------|------|
| A | 移除 stale banner | JS + CSS + server.py | UI 简洁 |
| B | 修复 git clone | server.py | 解决 repo 缺失 |
| C | commit 增量索引 | build_knowledge_index.py | 减少 Meilisearch upsert |
| D | 并行 git pull | server.py | 最大性能收益（-60%） |

---

## 性能预估

| 场景 | 当前 | 优化后 | 节省 |
|------|------|--------|------|
| 0 repo 变更 | ~45s | ~11s | **-76%** |
| 1-2 repo 变更（日常） | ~45s | ~13s | **-71%** |
| 全部 15 repo 变更 | ~45s | ~24s | **-47%** |

主要来源：Phase D 并行 pull 30s→8s（贡献 ~60%）；Phase C 增量 upsert 贡献 ~15-20%。

---

## Work Orders

| WO | Phase | 标题 | 依赖 |
|----|-------|------|------|
| WO-A1 | A | knowledge-search.js：移除 stale banner + 错误改 sync-bar | — |
| WO-A2 | A | app.css：删除 stale 样式 + 新增 ks-error 样式 | — |
| WO-A3 | A | server.py：移除 stale 字段 + MEILI_STALE_HOURS | — |
| WO-B1 | B | server.py：gh repo clone + shutil 清理 + failed_repos 报告 | — |
| WO-C1 | C | build_knowledge_index.py：filterable-attributes 设置 | — |
| WO-C2 | C | build_knowledge_index.py：commit cache 读写函数 | WO-C1 |
| WO-C3 | C | build_knowledge_index.py：main() 改为 per-repo 增量循环 | WO-C2 |
| WO-D1 | D | server.py：_sync_repo 函数 + ThreadPoolExecutor 并行化 | WO-B1 |

---

## 验证清单

1. **Phase A**：打开 viewer，触发 ↺，确认无 stale banner；触发失败，sync-bar 变红显示错误
2. **Phase B**：触发 ↺，server log 显示 `gh repo clone lulufoo/product-analysis …`；完成后 `/Users/lulu/Code/product-analysis/` 存在
3. **Phase C**：首次 ↺ 全量索引，所有 repo 显示 `[index]`；再次 ↺ 无修改，所有 repo 显示 `[skip] commit unchanged`
4. **Phase D**：计时 ↺，git pull 阶段 ~30s → ~8s
