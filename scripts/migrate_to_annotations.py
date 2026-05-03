#!/usr/bin/env python3
"""
One-time migration: move done/links from index.json entries → annotations/ files.

Usage:
    python3 scripts/migrate_to_annotations.py [--dry-run]
"""
import json
import os
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).parent.parent.resolve()
DRY_RUN = '--dry-run' in sys.argv


def annotation_path(common_path: str) -> Path:
    rel = (common_path[:-3] + '.json') if common_path.endswith('.md') else (common_path + '.json')
    return REPO_ROOT / 'annotations' / rel


def write_annotation(p: Path, data: dict):
    if DRY_RUN:
        print(f'  [dry-run] would write {p.relative_to(REPO_ROOT)}')
        print(f'    {json.dumps(data, ensure_ascii=False)}')
        return
    p.parent.mkdir(parents=True, exist_ok=True)
    tmp = p.with_suffix('.json.tmp')
    tmp.write_text(json.dumps(data, ensure_ascii=False, indent=2), encoding='utf-8')
    tmp.replace(p)


def main():
    index_path = REPO_ROOT / 'index.json'
    index_data = json.loads(index_path.read_text(encoding='utf-8'))
    entries = index_data.get('entries', index_data)

    migrated = 0
    cleaned_fields = []

    for entry_id, entry in entries.items():
        common_path = entry.get('common_path', '')
        if not common_path:
            continue

        ann_updates = {}
        if entry.get('done'):
            ann_updates['done'] = True
        if entry.get('links'):
            ann_updates['links'] = entry['links']

        if not ann_updates:
            continue

        p = annotation_path(common_path)

        # Merge with existing annotation if present
        existing = {}
        if p.exists():
            try:
                existing = json.loads(p.read_text(encoding='utf-8'))
            except Exception:
                pass

        merged = {**existing, **ann_updates}
        write_annotation(p, merged)

        # Mark fields for removal from index.json
        cleaned_fields.append(entry_id)
        migrated += 1
        print(f'  migrated {entry_id[:8]}… → {p.relative_to(REPO_ROOT)}')

    if not DRY_RUN and cleaned_fields:
        # Remove done/links from index.json entries
        for entry_id in cleaned_fields:
            entries[entry_id].pop('done', None)
            entries[entry_id].pop('links', None)

        tmp = index_path.with_suffix('.json.tmp')
        tmp.write_text(json.dumps(index_data, ensure_ascii=False, indent=2), encoding='utf-8')
        tmp.replace(index_path)
        print(f'\n  Cleaned done/links from {len(cleaned_fields)} index.json entries.')

    print(f'\nDone. {migrated} entries migrated.' + (' (dry-run)' if DRY_RUN else ''))


if __name__ == '__main__':
    main()
