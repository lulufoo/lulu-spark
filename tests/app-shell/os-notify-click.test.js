// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import * as api from '../../frontend/src/host/api.ts';
import { state } from '../../frontend/src/host/state.ts';
import { readLaterOpenStore } from '../../frontend/src/read-later/state/dialog-open.ts';
import { parseHash } from '../../frontend/src/router/index.ts';
import * as scheme from '../../frontend/src/router/scheme.ts';
import {
  handleOsNotifyClicked,
  startOsNotifyClickHub,
} from '../../frontend/src/app-shell/commands/os-notify-click.ts';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const NOTE_ID = 'abc';
const NOTE_PATH = 'inbox/x.md';
const NOTE_DATE = '20260719';
const NOTES_OPEN = `workbench://notes/open?id=${NOTE_ID}&path=${encodeURIComponent(NOTE_PATH)}`;
const READ_LATER_LIST = 'workbench://read-later/list';

function readRel(rel) {
  return readFileSync(join(repoRoot, rel), 'utf8');
}

function extractFunctionBody(source, name) {
  const start = source.indexOf(`function ${name}`);
  if (start === -1) return '';
  const braceStart = source.indexOf('{', start);
  let depth = 0;
  for (let i = braceStart; i < source.length; i += 1) {
    if (source[i] === '{') depth += 1;
    if (source[i] === '}') {
      depth -= 1;
      if (depth === 0) return source.slice(braceStart, i + 1);
    }
  }
  return '';
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

function flushListen() {
  return Promise.resolve();
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
  delete window.__TAURI__;
  vi.restoreAllMocks();
});

describe('handleOsNotifyClicked', () => {
  it('hands a notes/open scheme to L2 and opens that note', async () => {
    const openSpy = vi.spyOn(scheme, 'openWorkbenchScheme');
    await expect(handleOsNotifyClicked({ scheme: NOTES_OPEN })).resolves.toBe(true);
    expect(openSpy).toHaveBeenCalledTimes(1);
    expect(openSpy).toHaveBeenCalledWith(NOTES_OPEN);
    expect(parseHash(window.location.hash)).toEqual({
      name: 'workbench',
      params: { date: NOTE_DATE, note: NOTE_PATH },
    });
  });

  it('hands a read-later/list scheme to L2 and opens the list dialog', async () => {
    const openSpy = vi.spyOn(scheme, 'openWorkbenchScheme');
    await expect(handleOsNotifyClicked({ scheme: READ_LATER_LIST })).resolves.toBe(true);
    expect(openSpy).toHaveBeenCalledTimes(1);
    expect(openSpy).toHaveBeenCalledWith(READ_LATER_LIST);
    expect(dialogEl()?.classList.contains('open')).toBe(true);
    expect(window.location.hash).toBe('#/home');
  });

  it('returns false and does not land when payload has no scheme', async () => {
    const openSpy = vi.spyOn(scheme, 'openWorkbenchScheme');
    await expect(handleOsNotifyClicked({})).resolves.toBe(false);
    await expect(handleOsNotifyClicked({ scheme: undefined })).resolves.toBe(false);
    await expect(handleOsNotifyClicked({ scheme: '' })).resolves.toBe(false);
    await expect(handleOsNotifyClicked(undefined)).resolves.toBe(false);
    expect(openSpy).not.toHaveBeenCalled();
    expect(window.location.hash).toBe('#/home');
    expect(dialogEl()?.classList.contains('open')).toBe(false);
  });

  it.each([
    'https://notes/open?id=abc&path=inbox/x.md',
    'workbench://unknown/open?id=abc&path=inbox/x.md',
    'workbench://notes/open?id=abc',
    'workbench://notes/open?path=inbox/x.md',
    'workbench://read-later/open',
    'notes/open?id=abc&path=inbox/x.md',
  ])('forwards unrecognized or incomplete %s to L2 which ignores it', async (badScheme) => {
    const openSpy = vi.spyOn(scheme, 'openWorkbenchScheme');
    await expect(handleOsNotifyClicked({ scheme: badScheme })).resolves.toBe(false);
    expect(openSpy).toHaveBeenCalledWith(badScheme);
    expect(window.location.hash).toBe('#/home');
    expect(dialogEl()?.classList.contains('open')).toBe(false);
  });

  it('reloads Host index once and retries notes/open after a landing miss', async () => {
    state.index.data = {};
    vi.spyOn(api, 'fetchIndex').mockResolvedValue({
      entries: {
        [NOTE_ID]: {
          common_path: NOTE_PATH,
          created_at: `${NOTE_DATE}120000`,
        },
      },
    });
    await expect(handleOsNotifyClicked({ scheme: NOTES_OPEN })).resolves.toBe(true);
    expect(parseHash(window.location.hash)).toEqual({
      name: 'workbench',
      params: { date: NOTE_DATE, note: NOTE_PATH },
    });
  });
});

describe('startOsNotifyClickHub', () => {
  it('listens once for os-notification:clicked and lands the emitted scheme', async () => {
    /** @type {((event: { payload?: unknown }) => void) | undefined} */
    let handler;
    const listen = vi.fn(async (name, next) => {
      expect(name).toBe('os-notification:clicked');
      handler = next;
      return vi.fn();
    });
    window.__TAURI__ = { event: { listen } };
    const openSpy = vi.spyOn(scheme, 'openWorkbenchScheme');

    startOsNotifyClickHub();
    await flushListen();

    expect(listen).toHaveBeenCalledTimes(1);
    expect(typeof handler).toBe('function');
    await handler({ payload: { scheme: NOTES_OPEN } });
    expect(openSpy).toHaveBeenCalledWith(NOTES_OPEN);
    expect(parseHash(window.location.hash)).toEqual({
      name: 'workbench',
      params: { date: NOTE_DATE, note: NOTE_PATH },
    });
  });

  it('opens the Read Later dialog when the click payload is the list scheme', async () => {
    /** @type {((event: { payload?: unknown }) => void) | undefined} */
    let handler;
    window.__TAURI__ = {
      event: {
        listen: vi.fn(async (_name, next) => {
          handler = next;
          return vi.fn();
        }),
      },
    };

    startOsNotifyClickHub();
    await flushListen();
    await handler({ payload: { scheme: READ_LATER_LIST } });
    expect(dialogEl()?.classList.contains('open')).toBe(true);
    expect(window.location.hash).toBe('#/home');
  });

  it('does not register a second listen or double-land on a repeated start', async () => {
    /** @type {Array<(event: { payload?: unknown }) => void>} */
    const handlers = [];
    const listen = vi.fn(async (_name, next) => {
      handlers.push(next);
      return () => {
        const index = handlers.indexOf(next);
        if (index >= 0) handlers.splice(index, 1);
      };
    });
    window.__TAURI__ = { event: { listen } };
    const openSpy = vi.spyOn(scheme, 'openWorkbenchScheme');

    startOsNotifyClickHub();
    await flushListen();
    startOsNotifyClickHub();
    await flushListen();

    expect(handlers).toHaveLength(1);
    await handlers[0]({ payload: { scheme: NOTES_OPEN } });
    expect(openSpy).toHaveBeenCalledTimes(1);
    expect(parseHash(window.location.hash).name).toBe('workbench');
  });

  it('does not throw or change the UI when Tauri listen is missing', () => {
    delete window.__TAURI__;
    expect(() => startOsNotifyClickHub()).not.toThrow();
    expect(window.location.hash).toBe('#/home');
    expect(dialogEl()?.classList.contains('open')).toBe(false);
  });
});

describe('os-notify-click process-level wiring', () => {
  it('boots startOsNotifyClickHub from boot.ts without landing in boot', () => {
    const bootSrc = readRel('frontend/src/boot.ts');
    expect(bootSrc).toMatch(/startOsNotifyClickHub\s*\(\s*\)/);
    expect(bootSrc).toMatch(/os-notify-click/);
    expect(bootSrc).not.toMatch(/os-notification:clicked/);
    expect(bootSrc).not.toMatch(/handleOsNotifyClicked/);
    expect(bootSrc).not.toMatch(/openWorkbenchScheme/);
    expect(bootSrc).not.toMatch(/userInfo/);
  });

  it('consumes t3 { scheme } and hands it to L2 without composing or decoding userInfo', () => {
    const src = readRel('frontend/src/app-shell/commands/os-notify-click.ts');
    expect(src).toMatch(/os-notification:clicked/);
    expect(src).toMatch(/openWorkbenchScheme/);
    expect(src).toMatch(/from ['"].*router\/scheme\.ts['"]/);
    expect(src).not.toMatch(/userInfo/);
    expect(src).not.toMatch(/composeWorkbenchScheme/);
    expect(src).not.toMatch(/showOsNotification|show_os_notification/);
    expect(src).not.toMatch(/writeApiInvokeMap/);
    expect(src).not.toMatch(/@tauri-apps\//);
    expect(src).not.toMatch(/parseHash/);
    const body = extractFunctionBody(src, 'handleOsNotifyClicked');
    expect(body).toMatch(/openWorkbenchScheme/);
    expect(body).not.toMatch(/navigateToNote|openReadLaterDialog/);
  });
});
