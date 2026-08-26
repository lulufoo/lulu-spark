import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
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

const { makeEl, trigger, clearDom, qrMocks, invokeMock } = vi.hoisted(() => {
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

  const trigger = async (id, event, eventData = {}) => {
    const el = makeEl(id);
    for (const fn of el._listeners[event] || []) await fn(eventData);
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

  return { makeEl, trigger, clearDom, qrMocks, invokeMock };
});

vi.mock('../frontend/js/api.js', () => ({
  invoke: (...args) => invokeMock(...args),
}));

function seedDom() {
  makeEl('bind-dialog');
  makeEl('bind-preview');
  makeEl('bind-status');
  makeEl('bind-countdown');
  makeEl('btn-bind-close');
  makeEl('btn-bind-refresh').hidden = true;
  makeEl('qr-dialog');
  makeEl('qr-preview');
  makeEl('btn-qr');
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
  it('adds a Tools bind entry as a sibling of QR code and keeps Text-to-QR untouched', () => {
    const html = readSrc('frontend/index.html');
    const tools = extractById(html, 'tools-menu-dropdown');
    expect(tools).toMatch(/id="btn-qr"/);
    expect(tools).toMatch(/id="btn-bind"/);
    expect(tools.indexOf('id="btn-qr"')).toBeLessThan(tools.indexOf('id="btn-bind"'));
    expect(tools).toMatch(/id="btn-qr"[^>]*>📱 QR code/);
    expect(extractById(html, 'qr-dialog')).toContain('Text to QR code');
    expect(extractById(html, 'qr-dialog')).toMatch(/id="qr-preview"/);
    expect(extractById(html, 'qr-dialog')).not.toMatch(/id="bind-preview"/);
  });

  it('adds an independent bind overlay with its own preview node', () => {
    const html = readSrc('frontend/index.html');
    const overlay = extractById(html, 'bind-dialog');
    expect(overlay).toMatch(/id="bind-preview"/);
    expect(overlay).toMatch(/id="bind-status"/);
    expect(overlay).toMatch(/id="bind-countdown"/);
    expect(overlay).toMatch(/id="btn-bind-close"/);
    expect(overlay).toMatch(/id="btn-bind-refresh"/);
    expect(overlay).not.toMatch(/id="qr-preview"/);
    expect(overlay).not.toMatch(/id="qr-dialog"/);
    expect(html.match(/id="qr-dialog"/g) || []).toHaveLength(1);
    expect(html.match(/id="btn-qr"/g) || []).toHaveLength(1);
  });

  it('wires Tools bind entry to openBindDialog, not openQrDialog', () => {
    const mainJs = readSrc('frontend/js/main.js');
    expect(mainJs).toMatch(/from ['"]\.\/components\/modals\/bind-dialog\.js['"]/);
    expect(mainJs).toMatch(/openBindDialog/);
    const bindBlock = mainJs.match(
      /getElementById\(\s*['"]btn-bind['"]\s*\)[\s\S]{0,220}/,
    );
    expect(bindBlock, 'missing #btn-bind click wiring').toBeTruthy();
    expect(bindBlock[0]).toContain('openBindDialog');
    expect(bindBlock[0]).not.toContain('openQrDialog');
    const qrBlock = mainJs.match(
      /getElementById\(\s*['"]btn-qr['"]\s*\)[\s\S]{0,220}/,
    );
    expect(qrBlock, 'must keep #btn-qr → openQrDialog').toBeTruthy();
    expect(qrBlock[0]).toContain('openQrDialog');
  });

  it('does not reuse renderQr, qr-dialog, list_devices, or a frontend address', () => {
    const src = readSrc('frontend/js/components/modals/bind-dialog.js');
    const qrSrc = readSrc('frontend/js/components/modals/qr-dialog.js');
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
    expect(qrSrc).toContain("getElementById('qr-preview')");
    expect(qrSrc).toContain('export function renderQr');
    expect(qrSrc).toContain('export function openQrDialog');
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
    await import('../frontend/js/components/modals/bind-dialog.js');
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('does not issue or open on import (Host ready is not an auto-popup)', async () => {
    expect(makeEl('bind-dialog').classList.contains('open')).toBe(false);
    expect(issueCalls()).toHaveLength(0);
    expect(readCalls()).toHaveLength(0);
    expect(makeEl('qr-dialog').classList.contains('open')).toBe(false);
  });

  it('opens the independent overlay in loading first, then issues with no addresses', async () => {
    invokeMock.mockImplementation(async (cmd) => {
      if (cmd === 'issue_bind') {
        expect(makeEl('bind-dialog').classList.contains('open')).toBe(true);
        expect(makeEl('bind-status').textContent).toMatch(/Loading/);
        expect(qrMocks.toCanvas).not.toHaveBeenCalled();
        expect(makeEl('qr-dialog').classList.contains('open')).toBe(false);
        return PAYLOAD;
      }
      if (cmd === 'read_bind_session') return 'live';
      return undefined;
    });

    const { openBindDialog } = await import('../frontend/js/components/modals/bind-dialog.js');
    await openBindDialog();

    expect(issueCalls()).toHaveLength(1);
    expect(issueCalls()[0][0]).toBe('issue_bind');
    assertNoAddresses(issueCalls()[0]);
    expect(makeEl('bind-status').textContent).not.toMatch(/Loading/);
    expect(makeEl('qr-dialog').classList.contains('open')).toBe(false);
  });

  it('encodes the draw object as JSON and paints the bind preview via toCanvas + QR_OPTS', async () => {
    const { openBindDialog } = await import('../frontend/js/components/modals/bind-dialog.js');
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
    const { openBindDialog } = await import('../frontend/js/components/modals/bind-dialog.js');
    await openBindDialog();

    expect(makeEl('bind-countdown').textContent).toMatch(/180/);
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

    const { openBindDialog } = await import('../frontend/js/components/modals/bind-dialog.js');
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
    const { openBindDialog } = await import('../frontend/js/components/modals/bind-dialog.js');
    await openBindDialog();
    await trigger('btn-bind-close', 'click');
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

    const { openBindDialog } = await import('../frontend/js/components/modals/bind-dialog.js');
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

    const { openBindDialog } = await import('../frontend/js/components/modals/bind-dialog.js');
    await openBindDialog();
    await vi.advanceTimersByTimeAsync(1000);
    expect(makeEl('btn-bind-refresh').hidden).toBe(false);
    const readsAtRefresh = readCalls().length;

    session = 'live';
    const refresh = trigger('btn-bind-refresh', 'click');
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
      const { openBindDialog } = await import('../frontend/js/components/modals/bind-dialog.js');
      await openBindDialog();

      expect(makeEl('bind-dialog').classList.contains('open')).toBe(true);
      expect(makeEl('bind-status').textContent).not.toMatch(/Loading/);
      expect(qrMocks.toCanvas).not.toHaveBeenCalled();
      expect(makeEl('bind-preview').children).toHaveLength(0);
    }
  });
});
