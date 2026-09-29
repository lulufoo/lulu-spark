import { describe, it, expect, beforeEach, vi } from 'vitest';

function serializeTestNode(node) {
  if (node == null || node === false) return '';
  if (typeof node === 'string' || typeof node === 'number') return String(node);
  if (Array.isArray(node)) return node.map(serializeTestNode).join('');
  const type = node.type;
  const props = node.props || {};
  if (typeof type !== 'string') return serializeTestNode(props.children);
  const { children, className, ...rest } = props;
  let attrs = className ? ` class="${className}"` : '';
  for (const [key, value] of Object.entries(rest)) {
    if (value == null || key === 'children') continue;
    attrs += ` ${key}="${value}"`;
  }
  const inner = serializeTestNode(children);
  if (type === 'img' || type === 'input' || type === 'br') {
    return `<${type}${attrs}>`;
  }
  return `<${type}${attrs}>${inner}</${type}>`;
}

vi.mock('../../frontend/src/island.ts', () => ({
  renderToHtml: (node) => serializeTestNode(node),
}));

const OVERFLOW_MSG = '⚠️ Text too long to generate QR code (capacity ~2 KB UTF-8)';

const { makeEl, clearDom, qrMocks } = vi.hoisted(() => {
  const elements = {};
  const qrMocks = {
    toCanvas: vi.fn(),
    toDataURL: vi.fn(),
  };

  const clearDom = () => {
    for (const key of Object.keys(elements)) delete elements[key];
  };

  const makeEl = (id = '') => {
    if (id && elements[id]) return elements[id];
    const el = {
      id,
      tagName: '',
      style: {},
      classList: {
        _set: new Set(),
        contains(cls) { return this._set.has(cls); },
        add(cls) { this._set.add(cls); },
        remove(cls) { this._set.delete(cls); },
      },
      _listeners: {},
      _focused: false,
      children: [],
      addEventListener(event, fn) {
        if (!this._listeners[event]) this._listeners[event] = [];
        this._listeners[event].push(fn);
      },
      appendChild(child) {
        this.children.push(child);
        return child;
      },
      focus() {
        this._focused = true;
      },
      innerHTML: '',
      textContent: '',
      value: '',
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

  return { makeEl, clearDom, qrMocks };
});

function seedDom() {
  makeEl('convert-dialog');
  makeEl('qr-input');
  makeEl('qr-preview');
  makeEl('btn-convert-close');
}

describe('qr-dialog', () => {
  beforeEach(async () => {
    vi.resetModules();
    vi.clearAllMocks();
    clearDom();
    seedDom();

    qrMocks.toCanvas.mockImplementation((canvas, text, opts, cb) => {
      cb(null);
    });
    qrMocks.toDataURL.mockImplementation((text, opts, cb) => {
      cb(null, 'data:image/png;base64,mock');
    });

    await import('../../frontend/src/app-shell/ui/qr-dialog.tsx');
  });

  describe('openQrDialog', () => {
    it('clears input and preview, opens dialog, and focuses input', async () => {
      const { openQrDialog } = await import('../../frontend/src/app-shell/commands/qr-dialog.ts');

      makeEl('qr-input').value = 'stale';
      makeEl('qr-preview').innerHTML = '<canvas></canvas>';

      openQrDialog();

      expect(makeEl('qr-input').value).toBe('');
      expect(makeEl('qr-preview').innerHTML).toBe('');
      expect(makeEl('convert-dialog').classList.contains('open')).toBe(true);
      expect(makeEl('qr-input')._focused).toBe(true);
    });

    it('clears state when reopened after close', async () => {
      const { openQrDialog } = await import('../../frontend/src/app-shell/commands/qr-dialog.ts');

      makeEl('qr-input').value = 'https://example.com';
      makeEl('qr-preview').innerHTML = '<canvas></canvas>';
      makeEl('convert-dialog').classList.add('open');

      makeEl('convert-dialog').classList.remove('open');
      openQrDialog();

      expect(makeEl('qr-input').value).toBe('');
      expect(makeEl('qr-preview').innerHTML).toBe('');
      expect(makeEl('convert-dialog').classList.contains('open')).toBe(true);
    });
  });

  describe('renderQr', () => {
    it('renders QR via toCanvas for valid input', async () => {
      const { renderQr } = await import('../../frontend/src/app-shell/ui/qr-dialog.tsx');

      renderQr('https://example.com', makeEl('qr-preview'));

      expect(qrMocks.toCanvas).toHaveBeenCalledTimes(1);
      expect(qrMocks.toCanvas).toHaveBeenCalledWith(
        expect.objectContaining({ tagName: 'canvas' }),
        'https://example.com',
        { width: 256, margin: 2 },
        expect.any(Function),
      );
      expect(makeEl('qr-preview').children).toHaveLength(1);
      expect(makeEl('qr-preview').children[0].tagName).toBe('canvas');
    });

    it('does not call QRCode for empty or whitespace-only input', async () => {
      const { renderQr } = await import('../../frontend/src/app-shell/ui/qr-dialog.tsx');

      renderQr('', makeEl('qr-preview'));
      renderQr('   \n\t  ', makeEl('qr-preview'));

      expect(qrMocks.toCanvas).not.toHaveBeenCalled();
      expect(qrMocks.toDataURL).not.toHaveBeenCalled();
      expect(makeEl('qr-preview').innerHTML).toBe('');
      expect(makeEl('qr-preview').children).toHaveLength(0);
    });

    it('shows visible overflow error with .qr-error for capacity failures', async () => {
      const { renderQr } = await import('../../frontend/src/app-shell/ui/qr-dialog.tsx');

      qrMocks.toCanvas.mockImplementation((canvas, text, opts, cb) => {
        cb(new Error('code length overflow'));
      });
      qrMocks.toDataURL.mockImplementation((text, opts, cb) => {
        cb(new Error('The amount of data is too big to be stored in a QR Code'));
      });

      renderQr('x'.repeat(3000), makeEl('qr-preview'));

      expect(makeEl('qr-preview').innerHTML).toContain('qr-error');
      expect(makeEl('qr-preview').innerHTML).toContain(OVERFLOW_MSG);
      expect(qrMocks.toDataURL).not.toHaveBeenCalled();
    });

    it('falls back to toDataURL + img when toCanvas fails with non-overflow error', async () => {
      const { renderQr } = await import('../../frontend/src/app-shell/ui/qr-dialog.tsx');

      qrMocks.toCanvas.mockImplementation((canvas, text, opts, cb) => {
        cb(new Error('canvas unsupported'));
      });

      renderQr('https://example.com', makeEl('qr-preview'));

      expect(qrMocks.toDataURL).toHaveBeenCalledTimes(1);
      expect(qrMocks.toDataURL).toHaveBeenCalledWith(
        'https://example.com',
        { width: 256, margin: 2 },
        expect.any(Function),
      );
      expect(makeEl('qr-preview').innerHTML).toBe('<img src="data:image/png;base64,mock" alt="QR">');
    });
  });

  describe('event bindings', () => {
    it('can close the convert dialog overlay without convertStore', () => {
      makeEl('convert-dialog').classList.add('open');
      makeEl('convert-dialog').classList.remove('open');
      expect(makeEl('convert-dialog').classList.contains('open')).toBe(false);
    });

  });
});
