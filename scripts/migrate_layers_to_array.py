#!/usr/bin/env python3
"""
One-time migration: convert per-layer booleans to a "layers" array in index.json.

v4 format:  "raw": true, "distilled": false, "diagnose": false, "digest": true, "trace": false
v5 format:  "layers": ["raw", "digest"]

Usage:
    python3 scripts/migrate_layers_to_array.py [--dry-run]
"""
import json
import sys
from pathlib import Path

REPO_ROOT = Path(__file__).parent.parent.resolve()
INDEX_PATH = REPO_ROOT / 'index.json'
DRY_RUN = '--dry-run' in sys.argv

LAYER_ORDER = ['raw', 'distilled', 'diagnose', 'digest', 'trace']


def main():
    data = json.loads(INDEX_PATH.read_text(encoding='utf-8'))
    version = data.get('version', 0)

    if version == 5:
        print('Already version 5, nothing to do.')
        return
    if version not in (3, 4):
        print(f'Unsupported version {version}, expected 3 or 4.')
        sys.exit(1)

    entries = data.get('entries', {})
    migrated = 0

    for entry_id, entry in entries.items():
        layers = [l for l in LAYER_ORDER if entry.get(l) is True]
        # Remove boolean fields
        for l in LAYER_ORDER:
            entry.pop(l, None)
        entry['layers'] = layers
        migrated += 1
        if DRY_RUN:
            print(f'  [dry-run] {entry.get("common_path","?")[:60]} → layers: {layers}')

    data['version'] = 5

    if not DRY_RUN:
        tmp = INDEX_PATH.with_suffix('.json.tmp')
        tmp.write_text(json.dumps(data, ensure_ascii=False, indent=2), encoding='utf-8')
        tmp.replace(INDEX_PATH)
        print(f'Done. {migrated} entries migrated to version 5.')
    else:
        print(f'\nDone (dry-run). {migrated} entries would be migrated.')


if __name__ == '__main__':
    main()
