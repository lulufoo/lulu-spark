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

  it('lays the steps out below the code, not inside the right-hand meta column', () => {
    const steps = document.getElementById('bind-steps');
    expect(steps).not.toBeNull();
    expect(steps.querySelectorAll('li')).toHaveLength(3);
    expect(document.getElementById('bind-dialog-meta').contains(steps)).toBe(false);
    const body = document.getElementById('bind-dialog-body');
    const footer = document.getElementById('bind-dialog-footer');
    expect(body.compareDocumentPosition(steps) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
    expect(steps.compareDocumentPosition(footer) & Node.DOCUMENT_POSITION_FOLLOWING).toBeTruthy();
  });

  it('keeps the QR card a fixed square and scales the canvas by width', () => {
    const css = readRel('frontend/app.css');
    expect(css).toMatch(/#bind-qr-card\s*\{[^}]*width: 240px;[^}]*aspect-ratio: 1;/);
    expect(css).toMatch(/#bind-preview canvas,\s*#bind-preview img\s*\{[^}]*width: 100% !important;[^}]*height: auto !important;/);
    expect(css).toMatch(/#bind-steps\s*\{[^}]*grid-template-columns: repeat\(3, minmax\(0, 1fr\)\)/);
    expect(css).not.toMatch(/#bind-steps li\s*\{[^}]*(background|border):/);
  });

  it('keeps the dialog-only CSS (overlay, header) removed', () => {
    const css = readRel('frontend/app.css');
    expect(css).not.toMatch(/#bind-dialog\s*\{/);
    expect(css).not.toMatch(/#bind-dialog-header/);
    expect(css).toMatch(/#bind-dialog-box\s*\{[^}]*flex-direction: column/);
  });
});
