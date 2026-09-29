// @vitest-environment jsdom
import { createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { flushSync } from 'react-dom';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

const qrMocks = vi.hoisted(() => ({
  toCanvas: vi.fn(),
  toDataURL: vi.fn(),
}));

globalThis.QRCode = qrMocks;

import { ConvertDialog } from '../../frontend/src/app-shell/ui/convert-dialog.tsx';
import { convertStore } from '../../frontend/src/app-shell/state/convert.ts';

function closeConvertDialog() {
  convertStore.set((s) => ({ ...s, open: false }));
}

function openConvertDialog(tab = 'base64') {
  const name = tab === 'qr' ? 'qr' : 'base64';
  convertStore.set({ open: true, tab: name });
}

describe('ConvertDialog QR tab', () => {
  let root;

  beforeEach(() => {
    qrMocks.toCanvas.mockImplementation((canvas, text, opts, cb) => {
      cb(null);
    });
    qrMocks.toDataURL.mockImplementation((text, opts, cb) => {
      cb(null, 'data:image/png;base64,mock');
    });
    qrMocks.toCanvas.mockClear();
    qrMocks.toDataURL.mockClear();
    document.body.innerHTML = '<div id="host"></div>';
    root = createRoot(document.getElementById('host'));
    flushSync(() => root.render(createElement(ConvertDialog)));
    flushSync(() => closeConvertDialog());
  });

  afterEach(() => {
    flushSync(() => closeConvertDialog());
    flushSync(() => root.unmount());
    document.body.innerHTML = '';
  });

  it('paints QR into #qr-preview from the controlled input', () => {
    flushSync(() => openConvertDialog('qr'));
    const input = document.getElementById('qr-input');
    expect(input).toBeTruthy();
    flushSync(() => {
      input.value = 'https://example.com';
      input.dispatchEvent(new Event('input', { bubbles: true }));
    });
    expect(qrMocks.toCanvas).toHaveBeenCalledTimes(1);
    expect(qrMocks.toCanvas.mock.calls[0][1]).toBe('https://example.com');
    expect(document.querySelector('#qr-preview canvas')).toBeTruthy();
  });

  it('does not call QRCode for empty input', () => {
    flushSync(() => openConvertDialog('qr'));
    const input = document.getElementById('qr-input');
    flushSync(() => {
      input.value = '   ';
      input.dispatchEvent(new Event('input', { bubbles: true }));
    });
    expect(qrMocks.toCanvas).not.toHaveBeenCalled();
  });
});
