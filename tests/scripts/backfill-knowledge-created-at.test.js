import { execFileSync } from 'node:child_process';
import { mkdirSync, mkdtempSync, readdirSync, rmSync, utimesSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';

const script = join(dirname(fileURLToPath(import.meta.url)), '../../scripts/backfill_knowledge_created_at.py');

const SETUP = `
import sqlite3, sys
conn = sqlite3.connect(sys.argv[1] + '/spark-data.sqlite')
conn.executescript("""
CREATE TABLE workspace (id TEXT PRIMARY KEY, doc_table TEXT NOT NULL);
CREATE TABLE project (id TEXT PRIMARY KEY, workspace_id TEXT, folder TEXT NOT NULL);
CREATE TABLE doc_knowledge (doc_id TEXT PRIMARY KEY, raw_path TEXT NOT NULL, created_at TEXT, project_id TEXT NOT NULL);
CREATE TABLE doc_default (doc_id TEXT PRIMARY KEY, raw_path TEXT NOT NULL, created_at TEXT, project_id TEXT NOT NULL);
INSERT INTO workspace VALUES ('knowledge', 'doc_knowledge'), ('default', 'doc_default');
INSERT INTO project VALUES ('p1', 'knowledge', 'android'), ('p2', 'default', 'inbox');
INSERT INTO doc_knowledge VALUES
  ('k1', 'a/one.md', NULL, 'p1'),
  ('k2', 'two.md', '', 'p1'),
  ('k3', 'three.md', '202001010000', 'p1'),
  ('k4', 'gone.md', NULL, 'p1');
INSERT INTO doc_default VALUES ('d1', 'notes/x.md', NULL, 'p2');
""")
conn.commit()
`;

function run(dir, ...args) {
  return execFileSync('python3', [script, '--data-dir', dir, ...args], { encoding: 'utf8' });
}

function query(dir, sql) {
  const code = `import sqlite3,sys,json;print(json.dumps(sqlite3.connect(sys.argv[1]+'/spark-data.sqlite').execute(sys.argv[2]).fetchall()))`;
  return JSON.parse(execFileSync('python3', ['-c', code, dir, sql], { encoding: 'utf8' }));
}

function touch(dir, rel, iso) {
  const file = join(dir, 'workspaces/knowledge/raw/android', rel);
  mkdirSync(dirname(file), { recursive: true });
  writeFileSync(file, '# doc\n');
  const when = new Date(iso);
  utimesSync(file, when, when);
}

describe('backfill_knowledge_created_at.py', () => {
  let dir;

  beforeEach(() => {
    dir = mkdtempSync(join(tmpdir(), 'backfill-'));
    execFileSync('python3', ['-c', SETUP, dir]);
    touch(dir, 'a/one.md', '2026-04-23T10:15:00');
    touch(dir, 'two.md', '2026-05-07T08:05:00');
    touch(dir, 'three.md', '2026-06-01T00:00:00');
  });

  afterEach(() => rmSync(dir, { recursive: true, force: true }));

  it('dry run reports the plan and writes nothing', () => {
    const out = run(dir);
    expect(out).toContain('2 row(s) to fill, 1 without a file');
    expect(out).toContain('20260423: 1');
    expect(out).toContain('20260507: 1');
    expect(out).toContain('gone.md');
    expect(query(dir, "SELECT count(*) FROM doc_knowledge WHERE created_at IS NULL OR created_at = ''")).toEqual([[3]]);
    expect(readdirSync(dir)).not.toContain('_backup');
  });

  it('apply backs up, fills only the empty rows in 12-digit local time, and leaves the rest alone', () => {
    const out = run(dir, '--apply');
    expect(out).toContain('updated 2 row(s)');
    const rows = Object.fromEntries(query(dir, 'SELECT doc_id, created_at FROM doc_knowledge'));
    expect(rows.k1).toBe('202604231015');
    expect(rows.k2).toBe('202605070805');
    expect(rows.k3).toBe('202001010000');
    expect(rows.k4).toBeNull();
    expect(query(dir, "SELECT created_at FROM doc_default WHERE doc_id = 'd1'")).toEqual([[null]]);
    const backups = readdirSync(join(dir, '_backup'));
    expect(backups).toHaveLength(1);
    expect(backups[0]).toMatch(/^spark-data-before-created-at-backfill-\d{14}\.sqlite$/);
  });

  it('a second apply changes nothing', () => {
    run(dir, '--apply');
    const again = run(dir, '--apply');
    expect(again).toContain('0 row(s) to fill, 1 without a file');
    expect(again).toContain('nothing to change');
    expect(readdirSync(join(dir, '_backup'))).toHaveLength(1);
  });
});
