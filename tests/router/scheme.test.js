// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import {
  decodePassBag,
  encodePassBag,
  extractPassIdFromScheme,
  PASS_QUERY,
} from '../../frontend/src/auth/pass.ts';
import * as appLog from '../../frontend/src/host/app-log.ts';
import { state } from '../../frontend/src/host/state.ts';
import { readLaterOpenStore } from '../../frontend/src/read-later/state/dialog-open.ts';
import { parseHash } from '../../frontend/src/router/index.ts';
import { rememberNotifyTrace } from '../../frontend/src/router/notify-trace.ts';
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
const HOP_ID = 'trace_12345678';
const PASS = encodePassBag(HOP_ID);
const NOTES_OPEN = `spark://notes/open?id=${NOTE_ID}&path=${encodeURIComponent(NOTE_PATH)}&${PASS_QUERY}=${PASS}`;
const READ_LATER_LIST = `spark://read-later/list?${PASS_QUERY}=${PASS}`;
const NOTES_OPEN_NO_PASS = `spark://notes/open?id=${NOTE_ID}&path=${encodeURIComponent(NOTE_PATH)}`;
const NOTES_OPEN_TRACE_ONLY = `${NOTES_OPEN_NO_PASS}&trace=trace_12345678`;
const NOTES_OPEN_BAD_PASS = `${NOTES_OPEN_NO_PASS}&${PASS_QUERY}=not-json`;
const READ_LATER_TRACE_ONLY = 'spark://read-later/list?trace=trace_12345678';

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

function notesEnvelope(action = 'create', params = {}) {
  return {
    business: 'notes',
    action,
    params: { id: HOP_ID, archive_id: NOTE_ID, common_path: NOTE_PATH, ...params },
  };
}

function queryKeys(scheme) {
  return [...new URL(scheme).searchParams.keys()];
}

function passBagKeys(scheme) {
  const raw = new URL(scheme).searchParams.get(PASS_QUERY);
  const parsed = JSON.parse(decodeURIComponent(raw ?? ''));
  return Object.keys(parsed);
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
  it('encodes notes create as notes/open with id, path, and a pass bag from hop id', () => {
    const scheme = composeSparkScheme(notesEnvelope('create'));
    expect(scheme).toBe(NOTES_OPEN);
    expect(queryKeys(scheme)).toEqual(['id', 'path', 'pass']);
    expect(scheme).not.toMatch(/trace=/);
    expect(scheme).not.toMatch(/archive_id=/);
    expect(scheme).not.toMatch(/date=/);
    expect(scheme).not.toMatch(NOTE_DATE);
    expect(extractPassIdFromScheme(scheme)).toBe(HOP_ID);
    expect(passBagKeys(scheme)).toEqual(['id']);
    expect(decodePassBag(new URL(scheme).searchParams.get(PASS_QUERY))).toBe(HOP_ID);
  });

  it('encodes notes update as the same notes/open form', () => {
    expect(composeSparkScheme(notesEnvelope('update'))).toBe(NOTES_OPEN);
  });

  it('encodes read_later create as the list scheme with only a pass bag', () => {
    const scheme = composeSparkScheme({
      business: 'read_later',
      action: 'create',
      params: { id: HOP_ID, entry_id: 'e1' },
    });
    expect(scheme).toBe(READ_LATER_LIST);
    expect(queryKeys(scheme)).toEqual(['pass']);
    expect(scheme).not.toMatch(/trace=/);
    expect(extractPassIdFromScheme(scheme)).toBe(HOP_ID);
    expect(passBagKeys(scheme)).toEqual(['id']);
  });

  it('does not write a trace query when the envelope still carries trace_id', () => {
    const notes = composeSparkScheme(notesEnvelope('create', { trace_id: 'trace_12345678' }));
    expect(notes).toBe(NOTES_OPEN);
    expect(notes).not.toMatch(/trace=/);
    const later = composeSparkScheme({
      business: 'read_later',
      action: 'create',
      params: { id: HOP_ID, trace_id: 'trace_12345678' },
    });
    expect(later).toBe(READ_LATER_LIST);
    expect(later).not.toMatch(/trace=/);
  });

  it.each([
    [{ business: 'todos', action: 'create', params: { id: HOP_ID } }],
    [{ business: 'notes', action: 'changed', params: { id: HOP_ID, archive_id: NOTE_ID, common_path: NOTE_PATH } }],
    [{ business: 'notes', action: 'create', params: { archive_id: NOTE_ID, common_path: NOTE_PATH } }],
    [{ business: 'notes', action: 'create', params: { id: HOP_ID, common_path: NOTE_PATH } }],
    [{ business: 'notes', action: 'create', params: { id: HOP_ID, archive_id: NOTE_ID } }],
    [{ business: 'notes', action: 'create', params: { id: '', archive_id: NOTE_ID, common_path: NOTE_PATH } }],
    [{ business: 'notes', action: 'create', params: { id: HOP_ID, archive_id: '', common_path: NOTE_PATH } }],
    [{ business: 'notes', action: 'create', params: { id: HOP_ID, archive_id: NOTE_ID, common_path: '' } }],
    [{ business: 'notes', action: 'create', params: { id: NOTE_ID, common_path: NOTE_PATH } }],
    [{ business: 'read_later', action: 'update', params: { id: HOP_ID } }],
    [{ business: 'read_later', action: 'create', params: {} }],
  ])('returns null for rejected envelope %j', (envelope) => {
    expect(composeSparkScheme(envelope)).toBeNull();
  });
});

describe('parseSparkScheme', () => {
  it('parses notes/open id and path when pass is valid', () => {
    expect(parseSparkScheme(NOTES_OPEN)).toEqual({
      kind: 'notes-open',
      id: NOTE_ID,
      path: NOTE_PATH,
    });
  });

  it('parses read-later/list when pass is valid', () => {
    expect(parseSparkScheme(READ_LATER_LIST)).toEqual({
      kind: 'read-later-list',
    });
  });

  it('parses auth-login/callback with a code', () => {
    expect(parseSparkScheme('spark://auth-login/callback?code=abc')).toEqual({
      kind: 'auth-login-callback',
      code: 'abc',
      error: null,
    });
  });

  it('parses auth-login/callback with no query as null code and error', () => {
    expect(parseSparkScheme('spark://auth-login/callback')).toEqual({
      kind: 'auth-login-callback',
      code: null,
      error: null,
    });
  });

  it('parses auth-login/callback error without a code', () => {
    expect(parseSparkScheme('spark://auth-login/callback?error=access_denied')).toEqual({
      kind: 'auth-login-callback',
      code: null,
      error: 'access_denied',
    });
  });

  it('round-trips a path that contains spaces and non-ASCII', () => {
    const path = 'inbox/my 笔记.md';
    const scheme = composeSparkScheme(notesEnvelope('create', { common_path: path }));
    expect(scheme).toBe(
      `spark://notes/open?id=${NOTE_ID}&path=${encodeURIComponent(path)}&${PASS_QUERY}=${PASS}`,
    );
    expect(scheme).not.toMatch(/ /);
    expect(scheme).not.toMatch(/笔记/);
    expect(scheme).not.toMatch(/trace=/);
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
    'https://auth-login/callback?code=abc',
    'spark://auth-login/open',
    'spark://%',
    '',
    NOTES_OPEN_NO_PASS,
    NOTES_OPEN_TRACE_ONLY,
    NOTES_OPEN_BAD_PASS,
    `${NOTES_OPEN}&trace=trace_99999999zzzz`,
    'spark://read-later/list',
    READ_LATER_TRACE_ONLY,
    `${READ_LATER_LIST}&trace=trace_99999999zzzz`,
  ])('returns null for unrecognized, leftover, or incomplete scheme %s', (scheme) => {
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
  it('lands notes/open on #/spark and logs notes route.to_business from pass', () => {
    const logSpy = vi.spyOn(appLog, 'logAppEvent').mockImplementation(() => {});
    expect(openSparkScheme(NOTES_OPEN)).toBe(true);
    expect(parseHash(window.location.hash)).toEqual({
      name: 'spark',
      params: { date: NOTE_DATE, note: NOTE_PATH },
    });
    expect(dialogEl()?.classList.contains('open')).toBe(false);
    expect(logSpy).toHaveBeenCalledWith(
      expect.objectContaining({
        business: 'notes',
        event: 'route.to_business',
        traceId: HOP_ID,
        params: expect.objectContaining({ outcome: 'ok', kind: 'notes-open' }),
      }),
    );
    expect(logSpy.mock.calls.some(([input]) => input.business === 'os-notify')).toBe(false);
    logSpy.mockRestore();
  });

  it('opens the Read Later dialog and logs read_later route.to_business from pass', () => {
    const logSpy = vi.spyOn(appLog, 'logAppEvent').mockImplementation(() => {});
    expect(openSparkScheme(READ_LATER_LIST)).toBe(true);
    expect(dialogEl()?.classList.contains('open')).toBe(true);
    expect(window.location.hash).toBe('#/home');
    expect(logSpy).toHaveBeenCalledWith(
      expect.objectContaining({
        business: 'read_later',
        event: 'route.to_business',
        traceId: HOP_ID,
        params: expect.objectContaining({ outcome: 'ok', kind: 'read-later-list' }),
      }),
    );
    expect(logSpy.mock.calls.some(([input]) => input.business === 'os-notify')).toBe(false);
    logSpy.mockRestore();
  });

  it('does not land notes or throw for auth-login/callback', () => {
    expect(() => {
      expect(openSparkScheme('spark://auth-login/callback?code=abc')).toBe(true);
    }).not.toThrow();
    expect(parseHash(window.location.hash)).toEqual({ name: 'home', params: {} });
    expect(window.location.hash).toBe('#/home');
    expect(dialogEl()?.classList.contains('open')).toBe(false);
  });

  it.each([
    'https://notes/open?id=abc&path=inbox/x.md',
    'spark://unknown/open?id=abc&path=inbox/x.md',
    'spark://notes/open?id=abc',
    'spark://notes/open?path=inbox/x.md',
    'spark://read-later/open',
    'notes/open?id=abc&path=inbox/x.md',
    NOTES_OPEN_NO_PASS,
    NOTES_OPEN_TRACE_ONLY,
    NOTES_OPEN_BAD_PASS,
    'spark://read-later/list',
    READ_LATER_TRACE_ONLY,
  ])('ignores leftover or incomplete %s without opening notes or Read Later', (scheme) => {
    const logSpy = vi.spyOn(appLog, 'logAppEvent').mockImplementation(() => {});
    expect(parseSparkScheme(scheme)).toBeNull();
    expect(openSparkScheme(scheme)).toBe(false);
    expect(window.location.hash).toBe('#/home');
    expect(dialogEl()?.classList.contains('open')).toBe(false);
    expect(logSpy.mock.calls.some(([input]) => input.business === 'os-notify')).toBe(false);
    expect(
      logSpy.mock.calls.some(
        ([input]) => input.business === 'notes' || input.business === 'read_later',
      ),
    ).toBe(false);
    logSpy.mockRestore();
  });

  it('records leftover hops as app / missing and does not read ?trace=', () => {
    const logSpy = vi.spyOn(appLog, 'logAppEvent').mockImplementation(() => {});
    expect(openSparkScheme(NOTES_OPEN_TRACE_ONLY)).toBe(false);
    expect(openSparkScheme(READ_LATER_TRACE_ONLY)).toBe(false);
    expect(logSpy).toHaveBeenCalledWith(
      expect.objectContaining({
        business: 'app',
        event: 'route.to_business',
        traceId: 'trace_missing',
      }),
    );
    expect(logSpy.mock.calls.some(([input]) => input.traceId === 'trace_12345678')).toBe(false);
    expect(logSpy.mock.calls.some(([input]) => input.business === 'os-notify')).toBe(false);
    logSpy.mockRestore();
  });

  it('ignores a well-formed notes/open that the index cannot land', () => {
    state.index.data = {};
    expect(openSparkScheme(NOTES_OPEN)).toBe(false);
    expect(window.location.hash).toBe('#/home');
    expect(dialogEl()?.classList.contains('open')).toBe(false);
  });
});

describe('auth-login callback completion', () => {
  it('routes login without waiting, then writes exchange on the login business', async () => {
    const id = 'trace_loginhop01';
    const scheme = `spark://auth-login/callback?${PASS_QUERY}=${encodePassBag(id)}#access_token=secret`;
    rememberNotifyTrace('trace_notifyyyyy');
    const logSpy = vi.spyOn(appLog, 'logAppEvent').mockImplementation(() => {});
    expect(openSparkScheme(scheme)).toBe(true);
    expect(logSpy).toHaveBeenCalledWith(
      expect.objectContaining({
        business: 'login',
        event: 'route.to_business',
        traceId: id,
        params: expect.objectContaining({ outcome: 'ok', kind: 'auth-login-callback' }),
      }),
    );
    await vi.waitFor(() => {
      expect(logSpy).toHaveBeenCalledWith(
        expect.objectContaining({
          business: 'login',
          event: 'auth.exchange',
          traceId: id,
        }),
      );
    });
    expect(
      logSpy.mock.calls.some(
        ([input]) => input.business === 'os-notify' && input.event === 'route.to_business',
      ),
    ).toBe(false);
    expect(logSpy.mock.calls.some(([input]) => input.traceId === 'trace_notifyyyyy')).toBe(
      false,
    );
    expect(JSON.stringify(logSpy.mock.calls)).not.toContain('secret');
    logSpy.mockRestore();
  });

  it('writes login parse_fail for an auth-login path it does not recognize', () => {
    const logSpy = vi.spyOn(appLog, 'logAppEvent').mockImplementation(() => {});
    expect(openSparkScheme('spark://auth-login/open')).toBe(false);
    expect(logSpy).toHaveBeenCalledWith(
      expect.objectContaining({
        business: 'login',
        event: 'route.to_business',
        params: expect.objectContaining({ outcome: 'parse_fail' }),
      }),
    );
    logSpy.mockRestore();
  });

  it('hands auth-login-callback to completeAuthLogin without landing notes', () => {
    const src = readFileSync(join(repoRoot, 'frontend/src/router/scheme.ts'), 'utf8');
    expect(src).toMatch(/completeAuthLogin/);
    expect(src).toMatch(/from ['"].*auth\/oauth\.ts['"]/);
    expect(src).toMatch(/kind === 'auth-login-callback'/);
    expect(src).toMatch(/startSparkSchemeOpenedHub/);
    expect(src).toMatch(/spark-scheme:opened/);
    expect(src).not.toMatch(/linkIdentity/);
    expect(src).not.toMatch(/localStorage/);
  });

  it('boots the opened-scheme hub from boot.ts', () => {
    const bootSrc = readFileSync(join(repoRoot, 'frontend/src/boot.ts'), 'utf8');
    expect(bootSrc).toMatch(/startSparkSchemeOpenedHub/);
    expect(bootSrc).not.toMatch(/spark-scheme:opened/);
    expect(bootSrc).not.toMatch(/completeAuthLogin/);
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
