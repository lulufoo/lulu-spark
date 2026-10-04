// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { state } from '../../frontend/src/host/state.ts';
import { readLaterOpenStore } from '../../frontend/src/read-later/state/dialog-open.ts';
import { parseHash } from '../../frontend/src/router/index.ts';
import {
  composeSparkScheme,
  openSparkScheme,
  parseSparkScheme,
  resolveNotesLanding,
} from '../../frontend/src/router/scheme.ts';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const NOTE_ID = 'abc';
const NOTE_PATH = 'inbox/x.md';
const NOTE_DATE = '20260719';
const NOTES_OPEN = `spark://notes/open?id=${NOTE_ID}&path=${encodeURIComponent(NOTE_PATH)}`;

function collectBundleUrlSchemes(node, found = []) {
  if (Array.isArray(node)) {
    for (const item of node) collectBundleUrlSchemes(item, found);
    return found;
  }
  if (node && typeof node === 'object') {
    if (Array.isArray(node.CFBundleURLSchemes)) {
      found.push(...node.CFBundleURLSchemes);
    }
    for (const value of Object.values(node)) collectBundleUrlSchemes(value, found);
  }
  return found;
}

function seedIndex(entry = {
  _id: NOTE_ID,
  common_path: NOTE_PATH,
  created_at: `${NOTE_DATE}120000`,
}) {
  state.index.data = { [NOTE_ID]: entry };
}

function dialogEl() {
  return document.getElementById('read-later-dialog');
}

beforeEach(() => {
  window.location.hash = '#/home';
  document.body.innerHTML = '<div id="read-later-dialog"></div>';
  readLaterOpenStore.set(false);
  seedIndex();
});

afterEach(() => {
  state.index.data = null;
  readLaterOpenStore.set(false);
  document.body.innerHTML = '';
  window.location.hash = '#/home';
});

describe('composeSparkScheme', () => {
  it('encodes notes create as notes/open with id and path, without a date', () => {
    const scheme = composeSparkScheme({
      business: 'notes',
      action: 'create',
      params: { id: NOTE_ID, common_path: NOTE_PATH },
    });
    expect(scheme).toBe(NOTES_OPEN);
    expect(scheme).not.toMatch(/date=/);
    expect(scheme).not.toMatch(NOTE_DATE);
  });

  it('encodes notes update as the same notes/open form', () => {
    expect(
      composeSparkScheme({
        business: 'notes',
        action: 'update',
        params: { id: NOTE_ID, common_path: NOTE_PATH },
      }),
    ).toBe(NOTES_OPEN);
  });

  it('encodes read_later create as the list scheme', () => {
    expect(
      composeSparkScheme({
        business: 'read_later',
        action: 'create',
        params: { id: 'e1' },
      }),
    ).toBe('spark://read-later/list');
  });

  it.each([
    [{ business: 'todos', action: 'create', params: { id: 't1' } }],
    [{ business: 'notes', action: 'changed', params: { id: NOTE_ID, common_path: NOTE_PATH } }],
    [{ business: 'notes', action: 'create', params: { common_path: NOTE_PATH } }],
    [{ business: 'notes', action: 'create', params: { id: NOTE_ID } }],
    [{ business: 'notes', action: 'create', params: { id: '', common_path: NOTE_PATH } }],
    [{ business: 'notes', action: 'create', params: { id: NOTE_ID, common_path: '' } }],
    [{ business: 'read_later', action: 'update', params: { id: 'e1' } }],
  ])('returns null for rejected envelope %j', (envelope) => {
    expect(composeSparkScheme(envelope)).toBeNull();
  });
});

describe('parseSparkScheme', () => {
  it('parses notes/open id and path', () => {
    expect(parseSparkScheme(NOTES_OPEN)).toEqual({
      kind: 'notes-open',
      id: NOTE_ID,
      path: NOTE_PATH,
    });
  });

  it('parses read-later/list', () => {
    expect(parseSparkScheme('spark://read-later/list')).toEqual({
      kind: 'read-later-list',
    });
  });

  it('appends trace on notes/open and still parses id and path', () => {
    const scheme = composeSparkScheme({
      business: 'notes',
      action: 'create',
      params: { id: NOTE_ID, common_path: NOTE_PATH, trace_id: 'trace_12345678' },
    });
    expect(scheme).toBe(`${NOTES_OPEN}&trace=trace_12345678`);
    expect(parseSparkScheme(scheme)).toEqual({
      kind: 'notes-open',
      id: NOTE_ID,
      path: NOTE_PATH,
    });
  });

  it('parses read-later/list with only a trace query', () => {
    expect(parseSparkScheme('spark://read-later/list?trace=trace_12345678')).toEqual({
      kind: 'read-later-list',
    });
  });

  it('round-trips a path that contains spaces and non-ASCII', () => {
    const path = 'inbox/my 笔记.md';
    const scheme = composeSparkScheme({
      business: 'notes',
      action: 'create',
      params: { id: NOTE_ID, common_path: path },
    });
    expect(scheme).toBe(
      `spark://notes/open?id=${NOTE_ID}&path=${encodeURIComponent(path)}`,
    );
    expect(scheme).not.toMatch(/ /);
    expect(scheme).not.toMatch(/笔记/);
    expect(parseSparkScheme(scheme)).toEqual({
      kind: 'notes-open',
      id: NOTE_ID,
      path,
    });
  });

  it.each([
    'https://notes/open?id=abc&path=inbox/x.md',
    'notes/open?id=abc&path=inbox/x.md',
    'spark://unknown/open?id=abc&path=inbox/x.md',
    'spark://notes/list?id=abc&path=inbox/x.md',
    'spark://notes/open?id=abc',
    'spark://notes/open?path=inbox/x.md',
    'spark://notes/open?id=&path=inbox/x.md',
    'spark://notes/open?id=abc&path=',
    'spark://read-later/open',
    'spark://read-later/list?id=e1',
    '',
  ])('returns null for unrecognized or incomplete scheme %s', (scheme) => {
    expect(parseSparkScheme(scheme)).toBeNull();
  });
});

describe('resolveNotesLanding', () => {
  it('resolves date and common_path from index id', () => {
    expect(resolveNotesLanding(NOTE_ID, 'other/path.md')).toEqual({
      date: NOTE_DATE,
      note: NOTE_PATH,
    });
  });

  it('resolves date and common_path from index common_path', () => {
    expect(resolveNotesLanding('missing-id', NOTE_PATH)).toEqual({
      date: NOTE_DATE,
      note: NOTE_PATH,
    });
  });

  it('returns null when index cannot resolve date and common_path', () => {
    state.index.data = {};
    expect(resolveNotesLanding(NOTE_ID, NOTE_PATH)).toBeNull();
  });
});

describe('openSparkScheme', () => {
  it('lands notes/open on #/spark with resolved date and common_path', () => {
    expect(openSparkScheme(NOTES_OPEN)).toBe(true);
    expect(parseHash(window.location.hash)).toEqual({
      name: 'spark',
      params: { date: NOTE_DATE, note: NOTE_PATH },
    });
    expect(dialogEl()?.classList.contains('open')).toBe(false);
  });

  it('opens the Read Later dialog for read-later/list', () => {
    expect(openSparkScheme('spark://read-later/list')).toBe(true);
    expect(dialogEl()?.classList.contains('open')).toBe(true);
    expect(window.location.hash).toBe('#/home');
  });

  it.each([
    'https://notes/open?id=abc&path=inbox/x.md',
    'spark://unknown/open?id=abc&path=inbox/x.md',
    'spark://notes/open?id=abc',
    'spark://notes/open?path=inbox/x.md',
    'spark://read-later/open',
    'notes/open?id=abc&path=inbox/x.md',
  ])('ignores %s without changing hash or opening Read Later', (scheme) => {
    expect(parseSparkScheme(scheme)).toBeNull();
    expect(openSparkScheme(scheme)).toBe(false);
    expect(window.location.hash).toBe('#/home');
    expect(dialogEl()?.classList.contains('open')).toBe(false);
  });

  it('ignores a well-formed notes/open that the index cannot land', () => {
    state.index.data = {};
    expect(openSparkScheme(NOTES_OPEN)).toBe(false);
    expect(window.location.hash).toBe('#/home');
    expect(dialogEl()?.classList.contains('open')).toBe(false);
  });
});

describe('spark URL scheme registration', () => {
  it('registers spark on CFBundleURLSchemes and leaves parseHash hash landing in place', () => {
    const conf = JSON.parse(
      readFileSync(join(repoRoot, 'src-tauri/tauri.conf.json'), 'utf8'),
    );
    expect(collectBundleUrlSchemes(conf)).toContain('spark');
    expect(parseHash('#/spark?date=20260719&note=inbox/x.md')).toEqual({
      name: 'spark',
      params: { date: '20260719', note: 'inbox/x.md' },
    });
    expect(parseHash('#/read-later')).toEqual({ name: 'read-later', params: {} });
  });
});
