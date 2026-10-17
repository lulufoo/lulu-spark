// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { repoRoot } from '../helpers/read-frontend-js.js';

vi.mock('../../frontend/src/host/api.ts', () => ({
  invoke: vi.fn(),
}));

import { BindPanel } from '../../frontend/src/app-shell/ui/bind-dialog.tsx';

function visibleCopy(el) {
  return String(el?.textContent ?? '');
}

function readRel(rel) {
  return readFileSync(`${repoRoot}/${rel}`, 'utf8');
}

describe('Devices settings panel (pairing)', () => {
  let container;
  let root;

  beforeEach(() => {
    container = document.createElement('div');
    document.body.appendChild(container);
    root = createRoot(container);
    act(() => {
      root.render(createElement(BindPanel));
    });
  });

  afterEach(() => {
    act(() => {
      root?.unmount();
    });
    container?.remove();
    document.body.innerHTML = '';
  });

  it('is a settings panel carrying the pairing ids, with no overlay and no close button', () => {
    const panel = document.getElementById('settings-panel-devices');
    expect(panel).not.toBeNull();
    expect(panel.classList.contains('settings-panel')).toBe(true);
    for (const id of ['bind-dialog-box', 'bind-preview', 'bind-status', 'bind-countdown', 'btn-bind-refresh']) {
      expect(panel.querySelector(`#${id}`), id).not.toBeNull();
    }
    expect(document.getElementById('bind-dialog')).toBeNull();
    expect(document.getElementById('btn-bind-close')).toBeNull();
    expect(document.getElementById('bind-dialog-header')).toBeNull();
  });

  it('keeps the footer hint and the New code button', () => {
    expect(visibleCopy(document.getElementById('bind-dialog-footer'))).toMatch(/A new code is available/);
    expect(visibleCopy(document.getElementById('btn-bind-refresh'))).toMatch(/New code/);
    expect(visibleCopy(document.getElementById('bind-dialog-footer'))).not.toMatch(/Close/);
  });

  it('keeps the dialog-only CSS (overlay, header) removed', () => {
    const css = readRel('frontend/app.css');
    expect(css).not.toMatch(/#bind-dialog\s*\{/);
    expect(css).not.toMatch(/#bind-dialog-header/);
    expect(css).toMatch(/#bind-dialog-box\s*\{[^}]*flex-direction: column/);
  });
});
