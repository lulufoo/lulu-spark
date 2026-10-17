import { execFileSync } from 'node:child_process';
import { mkdtempSync, readdirSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const script = join(repoRoot, 'scripts/backfill_doc_source_type.py');

const SETUP = `
import sqlite3, sys
conn = sqlite3.connect(sys.argv[1] + '/spark-data.sqlite')
conn.executescript("""
CREATE TABLE workspace (id TEXT PRIMARY KEY, doc_table TEXT NOT NULL);
CREATE TABLE doc_knowledge (
  doc_id TEXT PRIMARY KEY,
  source_type TEXT,
  comments TEXT NOT NULL DEFAULT '[]',
  title TEXT
);
CREATE TABLE doc_default (
  doc_id TEXT PRIMARY KEY,
  source_type TEXT,
  comments TEXT NOT NULL DEFAULT '[]',
  title TEXT
);
CREATE TABLE doc_extra (
  doc_id TEXT PRIMARY KEY,
  source_type TEXT,
  comments TEXT NOT NULL DEFAULT '[]',
  title TEXT
);
INSERT INTO workspace VALUES ('default', 'doc_default'), ('extra', 'doc_extra'), ('knowledge', 'doc_knowledge');
INSERT INTO doc_knowledge VALUES
  ('k-null', NULL, '["keep-k"]', 'Knowledge null'),
  ('k-empty', '', '[]', 'empty string'),
  ('k-theme', 'theme-line', '[]', 'old video'),
  ('k-article', 'article', '[]', 'already article'),
  ('k-transcript', 'transcript', '[]', 'leave transcript');
INSERT INTO doc_default VALUES
  ('d-null', NULL, '["keep-d"]', 'Notes null stays'),
  ('d-theme', 'theme-line', '[]', 'Notes video'),
  ('d-summary', 'summary', '[]', 'summary stays'),
  ('d-transcript', 'transcript', '[]', 'Notes transcript');
INSERT INTO doc_extra VALUES
  ('e-theme', 'theme-line', '[]', 'third workspace'),
  ('e-jot', 'jot', '[]', 'jot stays');
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

function sourceMap(dir, table) {
  return Object.fromEntries(query(dir, `SELECT doc_id, source_type FROM ${table}`));
}

describe('backfill_doc_source_type.py', () => {
  let dir;

  beforeEach(() => {
    dir = mkdtempSync(join(tmpdir(), 'source-type-backfill-'));
    execFileSync('python3', ['-c', SETUP, dir]);
  });

  afterEach(() => rmSync(dir, { recursive: true, force: true }));

  it('dry run reports Knowledge NULL and theme-line counts and writes nothing', () => {
    const out = run(dir);
    expect(out).toContain('Knowledge NULL → article: 1');
    expect(out).toContain('theme-line → video: 3');
    expect(out).toContain('total: 4 row(s)');
    expect(out).toContain('dry run only');
    expect(sourceMap(dir, 'doc_knowledge')).toMatchObject({
      'k-null': null,
      'k-empty': '',
      'k-theme': 'theme-line',
      'k-transcript': 'transcript',
    });
    expect(sourceMap(dir, 'doc_default')['d-theme']).toBe('theme-line');
    expect(readdirSync(dir)).not.toContain('_backup');
  });

  it('apply backs up, maps NULL→article and theme-line→video, and leaves the rest alone', () => {
    const out = run(dir, '--apply');
    expect(out).toContain('updated 4 row(s)');
    expect(sourceMap(dir, 'doc_knowledge')).toEqual({
      'k-null': 'article',
      'k-empty': '',
      'k-theme': 'video',
      'k-article': 'article',
      'k-transcript': 'transcript',
    });
    expect(sourceMap(dir, 'doc_default')).toEqual({
      'd-null': null,
      'd-theme': 'video',
      'd-summary': 'summary',
      'd-transcript': 'transcript',
    });
    expect(sourceMap(dir, 'doc_extra')).toEqual({
      'e-theme': 'video',
      'e-jot': 'jot',
    });
    expect(query(dir, "SELECT comments FROM doc_knowledge WHERE doc_id = 'k-null'")).toEqual([['["keep-k"]']]);
    expect(query(dir, "SELECT comments FROM doc_default WHERE doc_id = 'd-null'")).toEqual([['["keep-d"]']]);
    const backups = readdirSync(join(dir, '_backup'));
    expect(backups).toHaveLength(1);
    expect(backups[0]).toMatch(/^spark-data-before-source-type-backfill-\d{14}\.sqlite$/);
  });

  it('a second apply changes nothing', () => {
    run(dir, '--apply');
    const again = run(dir, '--apply');
    expect(again).toContain('total: 0 row(s)');
    expect(again).toContain('nothing to change');
    expect(readdirSync(join(dir, '_backup'))).toHaveLength(1);
  });

  it('is not called from tauri or app boot', () => {
    const haystacks = [
      readFileSync(join(repoRoot, 'frontend/src/boot.ts'), 'utf8'),
      readFileSync(join(repoRoot, 'src-tauri/src/lib.rs'), 'utf8'),
      readFileSync(join(repoRoot, 'src-tauri/src/main.rs'), 'utf8'),
    ].join('\n');
    expect(haystacks).not.toMatch(/backfill_doc_source_type/);
  });
});
