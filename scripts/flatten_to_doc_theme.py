#!/usr/bin/env python3
"""
flatten_to_doc_theme.py — 全量迁移脚本

将所有文档从旧路径结构（最多4级）迁移到固定3级结构：
  <project>/<slug>/<ts>-<slug>.md

用法：
  python scripts/flatten_to_doc_theme.py --dry-run   # 只计算映射，不修改文件
  python scripts/flatten_to_doc_theme.py              # 执行实际迁移
"""

import sys
import os
import json
import re
import argparse
from pathlib import Path
from collections import defaultdict

ARCHIVE_ROOT = Path(__file__).parent.parent.resolve()
INDEX_PATH = ARCHIVE_ROOT / "index.json"
CACHE_DIR = ARCHIVE_ROOT / ".cache"


# ── slug 计算 ──────────────────────────────────────────────────────────────

def sanitize_slug(raw_slug: str) -> str:
    """将 slug 规范化为合法的目录名（kebab-case）。
    - 下划线 '_' 替换为连字符 '-'
    - 以 "数字." 开头（如 "3.aidl-..."）时，将首个 '.' 替换为 '-'
    """
    slug = raw_slug.replace("_", "-")
    slug = re.sub(r"^(\d+)\.", r"\1-", slug)
    return slug


def compute_new_cp(old_cp: str) -> str:
    parts = old_cp.split("/")
    project = parts[0]
    filename = parts[-1]                      # e.g. "202605010056-prompt-format-table-vs-text.md"
    ts = filename[:12]                        # "202605010056"
    raw_slug = filename[13:].removesuffix(".md")
    slug = sanitize_slug(raw_slug)
    return f"{project}/{slug}/{ts}-{slug}.md"


def compute_new_zh(old_zh: str) -> str:
    parts = old_zh.split("/")
    project = parts[0]
    filename = parts[-1]
    ts = filename[:12]
    raw_slug = filename[13:].removesuffix("-zh.md")
    slug = sanitize_slug(raw_slug)
    return f"{project}/{slug}/{ts}-{slug}-zh.md"


# ── 映射计算 ───────────────────────────────────────────────────────────────

def compute_mappings(index_data: dict) -> dict:
    entries = index_data.get("entries", index_data)

    entry_mappings = {}
    slug_map = defaultdict(list)  # (project, slug) -> [id]

    for entry_id, entry in entries.items():
        old_cp = entry["common_path"]
        new_cp = compute_new_cp(old_cp)

        parts = new_cp.split("/")
        project, slug = parts[0], parts[1]
        slug_map[(project, slug)].append(entry_id)

        mapping = {"old_cp": old_cp, "new_cp": new_cp}

        old_zh = entry.get("translations", {}).get("zh")
        if old_zh:
            mapping["old_zh"] = old_zh
            mapping["new_zh"] = compute_new_zh(old_zh)

        entry_mappings[entry_id] = mapping

    # 检测冲突
    conflicts = []
    for (project, slug), ids in slug_map.items():
        if len(ids) > 1:
            conflicts.append({"project": project, "slug": slug, "ids": ids})

    # diagnose 目录映射（扫描实际文件）
    diagnose_mappings = []
    diagnose_dir = ARCHIVE_ROOT / "diagnose"
    if diagnose_dir.exists():
        for f in diagnose_dir.rglob("*.md"):
            rel = f.relative_to(diagnose_dir).as_posix()
            new_rel = compute_new_cp(rel)
            diagnose_mappings.append({"old": f"diagnose/{rel}", "new": f"diagnose/{new_rel}"})

    # zh 映射（从 entry_mappings 中提取）
    zh_mappings = []
    for m in entry_mappings.values():
        if "old_zh" in m:
            zh_mappings.append({"old": f"raw/{m['old_zh']}", "new": f"raw/{m['new_zh']}"})

    return {
        "entries": entry_mappings,
        "conflicts": conflicts,
        "diagnose_mappings": diagnose_mappings,
        "zh_mappings": zh_mappings,
    }


# ── dry-run ────────────────────────────────────────────────────────────────

def run_dry(index_data: dict):
    mappings = compute_mappings(index_data)
    CACHE_DIR.mkdir(exist_ok=True)
    out_path = CACHE_DIR / "migration-preview.json"
    with open(out_path, "w", encoding="utf-8") as f:
        json.dump(mappings, f, ensure_ascii=False, indent=2)

    print(f"[dry-run] entries  : {len(mappings['entries'])}")
    print(f"[dry-run] conflicts: {len(mappings['conflicts'])}")
    print(f"[dry-run] diagnose : {len(mappings['diagnose_mappings'])}")
    print(f"[dry-run] zh files : {len(mappings['zh_mappings'])}")
    print(f"[dry-run] 输出: {out_path}")

    if mappings["conflicts"]:
        print("\n⚠️  存在冲突：")
        for c in mappings["conflicts"]:
            print(f"  {c['project']}/{c['slug']} : {c['ids']}")
        sys.exit(1)

    print("\n✅ 无冲突，可执行正式迁移")


# ── 正式迁移 ───────────────────────────────────────────────────────────────

def run_migrate(index_data: dict):
    mappings = compute_mappings(index_data)
    entries_map = mappings["entries"]

    # 前置检查
    if mappings["conflicts"]:
        print("❌ 存在冲突，中止迁移：")
        for c in mappings["conflicts"]:
            print(f"  {c['project']}/{c['slug']} : {c['ids']}")
        sys.exit(1)

    entries = index_data.get("entries", index_data)

    # Step 2: 层文件迁移（raw / distilled / digest / trace，跳过 diagnose）
    all_moves = []  # (src, dst, kind)
    for entry_id, m in entries_map.items():
        entry = entries[entry_id]
        old_cp, new_cp = m["old_cp"], m["new_cp"]
        for layer in ("raw", "distilled", "digest", "trace"):
            if layer not in entry.get("layers", []):
                continue
            src = ARCHIVE_ROOT / layer / old_cp
            dst = ARCHIVE_ROOT / layer / new_cp
            if src.exists():
                all_moves.append((src, dst, f"{layer}/{old_cp}"))

    # Step 3: diagnose 迁移
    for dm in mappings["diagnose_mappings"]:
        src = ARCHIVE_ROOT / dm["old"]
        dst = ARCHIVE_ROOT / dm["new"]
        if src.exists():
            all_moves.append((src, dst, dm["old"]))

    # Step 4: zh 文件迁移
    for entry_id, m in entries_map.items():
        if "old_zh" not in m:
            continue
        src = ARCHIVE_ROOT / "raw" / m["old_zh"]
        dst = ARCHIVE_ROOT / "raw" / m["new_zh"]
        if src.exists():
            all_moves.append((src, dst, f"raw/{m['old_zh']} [zh]"))

    # Step 5: annotations 重命名
    for entry_id, m in entries_map.items():
        old_cp, new_cp = m["old_cp"], m["new_cp"]
        old_ann = ARCHIVE_ROOT / "annotations" / (old_cp[:-3] + ".json")
        new_ann = ARCHIVE_ROOT / "annotations" / (new_cp[:-3] + ".json")
        if old_ann.exists():
            all_moves.append((old_ann, new_ann, f"annotations/{old_cp[:-3]}.json"))

    # 执行 mv（原子性：失败则 rollback）
    completed = []
    try:
        for src, dst, label in all_moves:
            dst.parent.mkdir(parents=True, exist_ok=True)
            src.rename(dst)
            completed.append((src, dst))
            print(f"  mv {label}")
    except Exception as e:
        print(f"\n❌ mv 失败：{e}，开始 rollback…")
        for dst_done, src_done in reversed([(s, d) for s, d in completed]):
            try:
                dst_done.rename(src_done)
                print(f"  rollback {dst_done.name}")
            except Exception as re_err:
                print(f"  rollback 失败: {re_err}")
        sys.exit(1)

    # Step 6: 更新文件内导航链接
    for entry_id, m in entries_map.items():
        old_cp, new_cp = m["old_cp"], m["new_cp"]
        old_zh = m.get("old_zh")
        new_zh = m.get("new_zh")
        # 对所有已迁移的 .md 文件做链接替换
        for layer in ("raw", "distilled", "digest", "trace", "diagnose"):
            candidate = ARCHIVE_ROOT / layer / new_cp
            if candidate.exists():
                _update_links(candidate, old_cp, new_cp, old_zh, new_zh)

    # Step 7: 写 index.json
    for entry_id, m in entries_map.items():
        entries[entry_id]["common_path"] = m["new_cp"]
        if "new_zh" in m:
            entries[entry_id]["translations"]["zh"] = m["new_zh"]

    tmp_path = INDEX_PATH.with_suffix(".json.tmp")
    with open(tmp_path, "w", encoding="utf-8") as f:
        json.dump(index_data, f, ensure_ascii=False, indent=2)
    tmp_path.replace(INDEX_PATH)
    print(f"\n✅ index.json 已更新")

    # Step 8: 清理旧空目录
    old_dirs = set()
    for entry_id, m in entries_map.items():
        for layer in ("raw", "distilled", "digest", "trace"):
            old_dir = (ARCHIVE_ROOT / layer / m["old_cp"]).parent
            old_dirs.add(old_dir)
        diag_src = ARCHIVE_ROOT / "diagnose" / m["old_cp"]
        old_dirs.add(diag_src.parent)

    for d in sorted(old_dirs, key=lambda p: -len(p.parts)):
        try:
            if d.exists() and not any(d.iterdir()):
                d.rmdir()
                print(f"  rmdir {d.relative_to(ARCHIVE_ROOT)}")
        except Exception:
            pass

    print(f"\n✅ 迁移完成，共移动 {len(completed)} 个文件")


def _update_links(md_path: Path, old_cp: str, new_cp: str, old_zh=None, new_zh=None):
    try:
        content = md_path.read_text(encoding="utf-8")
        original = content
        for layer in ("raw", "distilled", "digest", "trace", "diagnose"):
            content = content.replace(f"{layer}/{old_cp}", f"{layer}/{new_cp}")
            content = content.replace(f"`{layer}/{old_cp}`", f"`{layer}/{new_cp}`")
        if old_zh and new_zh:
            content = content.replace(f"raw/{old_zh}", f"raw/{new_zh}")
        if content != original:
            md_path.write_text(content, encoding="utf-8")
    except Exception as e:
        print(f"  ⚠️  链接更新失败 {md_path.name}: {e}")


# ── 入口 ───────────────────────────────────────────────────────────────────

def main():
    parser = argparse.ArgumentParser(description="Flatten archive to doc-theme structure")
    parser.add_argument("--dry-run", action="store_true", help="只计算映射，不修改文件")
    args = parser.parse_args()

    with open(INDEX_PATH, encoding="utf-8") as f:
        index_data = json.load(f)

    if args.dry_run:
        run_dry(index_data)
    else:
        run_migrate(index_data)


if __name__ == "__main__":
    main()
