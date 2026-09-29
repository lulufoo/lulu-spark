import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { readFrontendJs, readShellHtml } from '../helpers/read-frontend-js.js';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const shellHtml = readShellHtml();
const QR_OPTS = { width: 256, margin: 2 };
const PAYLOAD = {
  ip: '10.0.0.8',
  port: 7654,
  temp_pub: 'aa11',
  tls_fingerprint: 'ff22',
  exp: 1_700_000_180,
  sig: 'ss33',
};
const PAYLOAD_REFRESH = {
  ...PAYLOAD,
  temp_pub: 'bb22',
  exp: 1_700_000_360,
  sig: 'ss44',
};

function readSrc(rel) {
  if (rel.endsWith('.js') && rel.startsWith('frontend/src/')) return readFrontendJs(rel);
  return readFileSync(join(repoRoot, rel), 'utf8');
}

function extractById(html, id) {
  const openRe = new RegExp(`<([a-zA-Z0-9]+)([^>]*\\bid="${id}"[^>]*)>`, 'i');
  const open = openRe.exec(html);
  expect(open, `missing element #${id}`).toBeTruthy();
  const tag = open[1];
  const start = open.index;
  const afterOpen = start + open[0].length;
  if (/^(input|br|hr|img|meta|link)$/i.test(tag) || /\/>$/.test(open[0])) {
    return html.slice(start, afterOpen);
  }
  let depth = 1;
  const token = new RegExp(`</?${tag}\\b[^>]*>`, 'gi');
  token.lastIndex = afterOpen;
  let m;
  while ((m = token.exec(html)) !== null) {
    if (m[0].startsWith('</')) depth -= 1;
    else if (!/\/>$/.test(m[0])) depth += 1;
    if (depth === 0) return html.slice(start, m.index + m[0].length);
  }
  throw new Error(`unclosed #${id}`);
}

const { makeEl, clearDom, qrMocks, invokeMock } = vi.hoisted(() => {
  const elements = {};
  const qrMocks = { toCanvas: vi.fn() };
  const invokeMock = vi.fn();

  const clearDom = () => {
    for (const key of Object.keys(elements)) delete elements[key];
  };

  const makeEl = (id = '') => {
    if (id && elements[id]) return elements[id];
    const el = {
      id,
      tagName: '',
      style: {},
      hidden: false,
      classList: {
        _set: new Set(),
        contains(cls) { return this._set.has(cls); },
        add(cls) { this._set.add(cls); },
        remove(cls) { this._set.delete(cls); },
      },
      _listeners: {},
      children: [],
      addEventListener(event, fn) {
        if (!this._listeners[event]) this._listeners[event] = [];
        this._listeners[event].push(fn);
      },
      appendChild(child) {
        this.children.push(child);
        return child;
      },
      innerHTML: '',
      textContent: '',
    };
    if (id) elements[id] = el;
    return el;
  };

  globalThis.document = {
    getElementById: (id) => makeEl(id),
    createElement: (tag) => {
      const el = makeEl();
      el.tagName = tag;
      return el;
    },
    addEventListener: () => {},
  };
  globalThis.QRCode = qrMocks;

  return { makeEl, clearDom, qrMocks, invokeMock };
});

vi.mock('../../frontend/src/host/api.ts', () => ({
  invoke: (...args) => invokeMock(...args),
}));

function seedDom() {
  makeEl('bind-dialog');
  makeEl('bind-preview');
  makeEl('bind-status');
  makeEl('bind-countdown');
  makeEl('btn-bind-close');
  makeEl('btn-bind-refresh').hidden = true;
  makeEl('convert-dialog');
  makeEl('qr-preview');
}

function issueCalls() {
  return invokeMock.mock.calls.filter((c) => c[0] === 'issue_bind');
}

function readCalls() {
  return invokeMock.mock.calls.filter((c) => c[0] === 'read_bind_session');
}

function assertNoAddresses(call) {
  const args = call[1];
  if (args == null) return;
  expect(args).not.toHaveProperty('ip');
  expect(args).not.toHaveProperty('port');
  expect(args).not.toHaveProperty('address');
  expect(args).not.toHaveProperty('tls_fingerprint');
}

describe('bind-dialog markup and wiring', () => {
  it('keeps Bind device in the tools menu without a Convert entry', () => {
    const html = shellHtml;
    const tools = extractById(html, 'tools-menu-dropdown');
    expect(tools).toMatch(/id="btn-bind"/);
    expect(tools).not.toMatch(/id="btn-convert"/);
    expect(tools).not.toMatch(/id="btn-move-doc-header"/);
    expect(tools).toMatch(/id="btn-bind"[^>]*>[\s\S]*?Bind device/);
  });

  it('adds an independent bind overlay with its own preview node', () => {
    const html = shellHtml;
    const overlay = extractById(html, 'bind-dialog');
    expect(overlay).toMatch(/id="bind-preview"/);
    expect(overlay).toMatch(/id="bind-status"/);
    expect(overlay).toMatch(/id="bind-countdown"/);
    expect(overlay).toMatch(/id="btn-bind-close"/);
    expect(overlay).toMatch(/id="btn-bind-refresh"/);
    expect(overlay).not.toMatch(/id="qr-preview"/);
    expect(overlay).not.toMatch(/id="convert-dialog"/);
    expect(html.match(/id="convert-dialog"/g) || []).toHaveLength(0);
    expect(html.match(/id="qr-dialog"/g) || []).toHaveLength(0);
    expect(html.match(/id="btn-qr"/g) || []).toHaveLength(0);
  });

  it('wires the bind entry to openBindDialog', () => {
    const shell = readSrc('frontend/src/shell.tsx');
    expect(shell).toMatch(/from ['"]\.\/app-shell\/ui\/bind-dialog\.tsx['"]/);
    expect(shell).toMatch(/openBindDialog/);
    expect(shell).toMatch(/<BindDialog\s*\/>/);
    const bindBlock = shell.match(
      /id="btn-bind"[\s\S]{0,280}openBindDialog/,
    );
    expect(bindBlock, 'missing #btn-bind click wiring').toBeTruthy();
    expect(bindBlock[0]).toContain('openBindDialog');
    expect(bindBlock[0]).not.toContain('openQrDialog');
    expect(bindBlock[0]).not.toContain('openConvertDialog');
    expect(shell).not.toMatch(/\bopenConvertDialog\b/);
    expect(shell).not.toMatch(/\bopenMoveDocDialog\b/);
    expect(shell).not.toMatch(/\bConvertDialog\b/);
    expect(shell).not.toMatch(/\bMoveDocDialog\b/);
  });

  it('does not reuse renderQr, qr-dialog, list_devices, or a frontend address', () => {
    const src = [
      readSrc('frontend/src/app-shell/ui/bind-dialog.tsx'),
      readSrc('frontend/src/app-shell/commands/bind-dialog.ts'),
      readSrc('frontend/src/app-shell/state/dialog-open.ts'),
    ].join('\n');
    expect(src).toContain('QRCode.toCanvas');
    expect(src).toMatch(/QR_OPTS/);
    expect(src).toContain('issue_bind');
    expect(src).toContain('read_bind_session');
    expect(src).not.toMatch(/\brenderQr\b/);
    expect(src).not.toContain('create_bind_payload');
    expect(src).not.toContain('list_devices');
    expect(src).not.toMatch(/getElementById\(\s*['"]qr-preview['"]/);
    expect(src).not.toMatch(/getElementById\(\s*['"]qr-dialog['"]/);
    expect(src).not.toMatch(/getElementById\(\s*['"]btn-qr['"]/);
    expect(src).toMatch(/export function BindDialog/);
    expect(src).toMatch(/createModuleStore/);
    expect(src).toMatch(/useSyncExternalStore/);
    expect(src).toMatch(/id="btn-bind-close"[\s\S]*?onClick/);
    expect(src).toMatch(/id="btn-bind-refresh"[\s\S]*?onClick/);
    expect(src).not.toMatch(/addEventListener/);
  });
});

describe('bind-dialog', () => {
  beforeEach(async () => {
    vi.resetModules();
    vi.clearAllMocks();
    clearDom();
    seedDom();
    vi.useFakeTimers({
      toFake: ['setInterval', 'clearInterval', 'setTimeout', 'clearTimeout'],
    });
    qrMocks.toCanvas.mockImplementation((canvas, text, opts, cb) => {
      cb(null);
    });
    invokeMock.mockImplementation(async (cmd) => {
      if (cmd === 'issue_bind') return PAYLOAD;
      if (cmd === 'read_bind_session') return 'live';
      return undefined;
    });
    await import('../../frontend/src/app-shell/commands/bind-dialog.ts');
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('does not issue or open on import (Host ready is not an auto-popup)', async () => {
    expect(makeEl('bind-dialog').classList.contains('open')).toBe(false);
    expect(issueCalls()).toHaveLength(0);
    expect(readCalls()).toHaveLength(0);
    expect(makeEl('convert-dialog').classList.contains('open')).toBe(false);
  });

  it('opens the independent overlay in loading first, then issues with no addresses', async () => {
    invokeMock.mockImplementation(async (cmd) => {
      if (cmd === 'issue_bind') {
        expect(makeEl('bind-dialog').classList.contains('open')).toBe(true);
        expect(makeEl('bind-status').textContent).toMatch(/Loading/);
        expect(qrMocks.toCanvas).not.toHaveBeenCalled();
        expect(makeEl('convert-dialog').classList.contains('open')).toBe(false);
        return PAYLOAD;
      }
      if (cmd === 'read_bind_session') return 'live';
      return undefined;
    });

    const { openBindDialog } = await import('../../frontend/src/app-shell/commands/bind-dialog.ts');
    await openBindDialog();

    expect(issueCalls()).toHaveLength(1);
    expect(issueCalls()[0][0]).toBe('issue_bind');
    assertNoAddresses(issueCalls()[0]);
    expect(makeEl('bind-status').textContent).not.toMatch(/Loading/);
    expect(makeEl('convert-dialog').classList.contains('open')).toBe(false);
  });

  it('encodes the draw object as JSON and paints the bind preview via toCanvas + QR_OPTS', async () => {
    const { openBindDialog } = await import('../../frontend/src/app-shell/commands/bind-dialog.ts');
    await openBindDialog();

    expect(qrMocks.toCanvas).toHaveBeenCalledTimes(1);
    const [canvas, text, opts] = qrMocks.toCanvas.mock.calls[0];
    expect(canvas.tagName).toBe('canvas');
    expect(opts).toEqual(QR_OPTS);
    const drawn = JSON.parse(text);
    expect(Object.keys(drawn)).toEqual([
      'ip',
      'port',
      'temp_pub',
      'tls_fingerprint',
      'exp',
      'sig',
    ]);
    expect(drawn).toEqual(PAYLOAD);
    expect(makeEl('bind-preview').children).toHaveLength(1);
    expect(makeEl('bind-preview').children[0].tagName).toBe('canvas');
    expect(makeEl('qr-preview').children).toHaveLength(0);
  });

  it('uses payload exp for the countdown, not the read-session command', async () => {
    vi.spyOn(Date, 'now').mockReturnValue(1_700_000_000_000);
    const { openBindDialog } = await import('../../frontend/src/app-shell/commands/bind-dialog.ts');
    await openBindDialog();

    expect(makeEl('bind-countdown').textContent).toBe('Expires in 3:00');
    expect(readCalls().every((c) => c[0] === 'read_bind_session')).toBe(true);
    for (const call of readCalls()) {
      expect(call).toHaveLength(1);
    }
  });

  it('polls read_bind_session every 1s while open and shows English success on consumed', async () => {
    let session = 'live';
    invokeMock.mockImplementation(async (cmd) => {
      if (cmd === 'issue_bind') return PAYLOAD;
      if (cmd === 'read_bind_session') return session;
      return undefined;
    });

    const { openBindDialog } = await import('../../frontend/src/app-shell/commands/bind-dialog.ts');
    await openBindDialog();
    expect(readCalls()).toHaveLength(0);

    await vi.advanceTimersByTimeAsync(1000);
    expect(readCalls()).toHaveLength(1);
    expect(readCalls()[0]).toEqual(['read_bind_session']);
    expect(makeEl('bind-status').textContent).not.toMatch(/successful/i);

    session = 'consumed';
    await vi.advanceTimersByTimeAsync(1000);
    expect(readCalls()).toHaveLength(2);
    expect(makeEl('bind-status').textContent).toBe('Binding successful');
    expect(makeEl('bind-status').textContent).not.toMatch(/[\u4e00-\u9fff]/);

    await vi.advanceTimersByTimeAsync(3000);
    expect(readCalls()).toHaveLength(2);
  });

  it('stops polling on close', async () => {
    const { openBindDialog, closeBindDialog } = await import('../../frontend/src/app-shell/commands/bind-dialog.ts');
    await openBindDialog();
    closeBindDialog();
    expect(makeEl('bind-dialog').classList.contains('open')).toBe(false);

    const before = readCalls().length;
    await vi.advanceTimersByTimeAsync(3000);
    expect(readCalls()).toHaveLength(before);
  });

  it('hides the QR and offers refresh on expired without auto-reissue', async () => {
    let session = 'live';
    invokeMock.mockImplementation(async (cmd) => {
      if (cmd === 'issue_bind') return PAYLOAD;
      if (cmd === 'read_bind_session') return session;
      return undefined;
    });

    const { openBindDialog } = await import('../../frontend/src/app-shell/commands/bind-dialog.ts');
    await openBindDialog();
    expect(makeEl('bind-preview').children).toHaveLength(1);

    session = 'expired';
    await vi.advanceTimersByTimeAsync(1000);

    expect(makeEl('bind-preview').innerHTML).toBe('');
    expect(makeEl('bind-preview').children).toHaveLength(0);
    expect(makeEl('btn-bind-refresh').hidden).toBe(false);
    expect(issueCalls()).toHaveLength(1);
  });

  it('refresh stops polling, shows loading, and issues a new code', async () => {
    const payloads = [PAYLOAD, PAYLOAD_REFRESH];
    let session = 'expired';
    invokeMock.mockImplementation(async (cmd) => {
      if (cmd === 'issue_bind') return payloads.shift();
      if (cmd === 'read_bind_session') return session;
      return undefined;
    });

    const { openBindDialog } = await import('../../frontend/src/app-shell/commands/bind-dialog.ts');
    await openBindDialog();
    await vi.advanceTimersByTimeAsync(1000);
    expect(makeEl('btn-bind-refresh').hidden).toBe(false);
    const readsAtRefresh = readCalls().length;

    session = 'live';
    const refresh = openBindDialog();
    expect(makeEl('bind-status').textContent).toMatch(/Loading/);
    await refresh;

    expect(issueCalls()).toHaveLength(2);
    assertNoAddresses(issueCalls()[1]);
    const drawn = JSON.parse(qrMocks.toCanvas.mock.calls.at(-1)[1]);
    expect(drawn.temp_pub).toBe('bb22');
    expect(makeEl('bind-preview').children).toHaveLength(1);
    expect(makeEl('btn-bind-refresh').hidden).toBe(true);

    await vi.advanceTimersByTimeAsync(1000);
    expect(readCalls().length).toBeGreaterThan(readsAtRefresh);
  });

  it('ends loading and does not draw on no_lan or no_gateway', async () => {
    for (const err of ['no_lan', 'no_gateway']) {
      vi.clearAllMocks();
      clearDom();
      seedDom();
      invokeMock.mockImplementation(async (cmd) => {
        if (cmd === 'issue_bind') throw new Error(err);
        if (cmd === 'read_bind_session') return 'idle';
        return undefined;
      });
      qrMocks.toCanvas.mockImplementation((canvas, text, opts, cb) => cb(null));
      const { openBindDialog } = await import('../../frontend/src/app-shell/commands/bind-dialog.ts');
      await openBindDialog();

      expect(makeEl('bind-dialog').classList.contains('open')).toBe(true);
      expect(makeEl('bind-status').textContent).not.toMatch(/Loading/);
      expect(qrMocks.toCanvas).not.toHaveBeenCalled();
      expect(makeEl('bind-preview').children).toHaveLength(0);
    }
  });

  it('identifies issue failures with Bind_Mobile in the browser log', async () => {
    const log = vi.spyOn(console, 'info').mockImplementation(() => {});
    invokeMock.mockImplementation(async (cmd) => {
      if (cmd === 'issue_bind') throw new Error('keychain_unavailable');
      return undefined;
    });

    const { openBindDialog } = await import('../../frontend/src/app-shell/commands/bind-dialog.ts');
    await openBindDialog();

    expect(makeEl('bind-status').textContent).toBe('The local Keychain is unavailable.');
    expect(log).toHaveBeenCalledWith(
      '[bind] business_id=Bind_Mobile event=issue_bind outcome=keychain_unavailable',
    );
    log.mockRestore();
  });

  it('identifies QR render failures with Bind_Mobile in the browser log', async () => {
    const log = vi.spyOn(console, 'info').mockImplementation(() => {});
    qrMocks.toCanvas.mockImplementation((canvas, text, opts, cb) => cb(new Error('render failed')));

    const { openBindDialog } = await import('../../frontend/src/app-shell/commands/bind-dialog.ts');
    await openBindDialog();

    expect(makeEl('bind-status').textContent).toBe('Unable to generate a binding code.');
    expect(log).toHaveBeenCalledWith(
      '[bind] business_id=Bind_Mobile event=qr.render outcome=failed',
    );
    log.mockRestore();
  });
});
