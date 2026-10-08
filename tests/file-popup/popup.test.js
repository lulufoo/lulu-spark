// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { FilePopup } from '../../frontend/src/file-popup/ui/popup.tsx';
import { emptyFilePopupView, patchView, viewStore } from '../../frontend/src/file-popup/state/store.ts';
import { repoRoot } from '../helpers/read-frontend-js.js';

vi.mock('../../frontend/src/host/api.ts', () => ({
  fetchDocHighlights: vi.fn(async () => ({ highlights: [] })),
}));

const GLYPH = '✕';

function visibleCopy(el) {
  return String(el?.textContent ?? '');
}

function readRel(rel) {
  return readFileSync(`${repoRoot}/${rel}`, 'utf8');
}

describe('FilePopup chrome', () => {
  let container;
  let root;

  afterEach(() => {
    act(() => {
      root?.unmount();
    });
    container?.remove();
    viewStore.set(emptyFilePopupView());
  });

  function renderOpen(partial = {}) {
    container = document.createElement('div');
    document.body.appendChild(container);
    root = createRoot(container);
    patchView({
      open: true,
      path: '/kb/notes/raw/inbox/a.md',
      title: 'Sample note',
      identityKey: 'notes:inbox/a.md',
      content: '# Hello',
      editing: false,
      loading: false,
      saving: false,
      error: '',
      ...partial,
    });
    act(() => {
      root.render(createElement(FilePopup));
    });
  }

  it('shows Edit and Copy with the shared dismiss instead of a Close label', () => {
    const src = readRel('frontend/src/file-popup/ui/popup.tsx');
    expect(src).toMatch(/<OverlayDismissButton[\s\S]*?closeFilePopup/);

    renderOpen({
      path: '/Users/me/knowledge/demo/doc.md',
      title: 'Knowledge doc',
      identityKey: 'knowledge:demo/doc.md',
      content: '# KB',
    });
    const labels = [...container.querySelectorAll('button')].map((b) => b.textContent.trim());
    expect(labels).toContain('Edit');
    expect(labels).toContain('Copy');
    expect(labels).not.toContain('Close');

    const dismiss = container.querySelector('.overlay-dismiss-button');
    expect(dismiss).not.toBeNull();
    expect(visibleCopy(dismiss).trim()).toBe(GLYPH);
    expect(visibleCopy(dismiss)).not.toContain('Close');
  });

  it('Copy writes the open path', async () => {
    const writeText = vi.fn(async () => {});
    Object.defineProperty(navigator, 'clipboard', {
      configurable: true,
      value: { writeText },
    });
    renderOpen();
    const copyBtn = container.querySelector('[data-role="copy-file-path"]');
    expect(copyBtn).not.toBeNull();
    expect(copyBtn.disabled).toBe(false);
    await act(async () => {
      copyBtn.click();
      await Promise.resolve();
    });
    expect(writeText).toHaveBeenCalledWith('/kb/notes/raw/inbox/a.md');
  });

  it('Copy stays disabled without a path', () => {
    renderOpen({ path: '' });
    const copyBtn = container.querySelector('[data-role="copy-file-path"]');
    expect(copyBtn.disabled).toBe(true);
  });

  it('uses Notes viewer chrome classes instead of a global header', () => {
    renderOpen();
    const dialog = container.querySelector('#file-popup');
    expect(dialog).not.toBeNull();
    expect(dialog.querySelector('header')).toBeNull();
    expect(dialog.querySelector('.viewer-header')).not.toBeNull();
    expect(dialog.querySelector('.viewer-panel-title')?.textContent).toBe('Sample note');
    const body = dialog.querySelector('#file-popup-body');
    expect(body).not.toBeNull();
    expect(body.classList.contains('viewer-body')).toBe(true);
    expect(dialog.querySelector('#file-popup-edit-area')?.classList.contains('viewer-edit-area')).toBe(
      true,
    );
  });

  it('keeps File popup close on the shared dismiss', () => {
    renderOpen();
    expect(viewStore.getSnapshot().open).toBe(true);
    act(() => {
      container.querySelector('.overlay-dismiss-button').click();
    });
    expect(viewStore.getSnapshot().open).toBe(false);
  });

  it('disables the shared File popup dismiss while saving and does not close', () => {
    renderOpen({ saving: true });
    const dismiss = container.querySelector('.overlay-dismiss-button');
    expect(dismiss).not.toBeNull();
    expect(dismiss.disabled).toBe(true);
    act(() => {
      dismiss.click();
    });
    expect(viewStore.getSnapshot().open).toBe(true);
  });

  it('lifts the Stage file popup off the canvas with the shared modal stack', () => {
    const css = readRel('frontend/app.css');
    expect(css).toMatch(/#file-popup\s*\{[^}]*background:\s*var\(--bg-rgba-000000-55\)/);
    expect(css).toMatch(
      /#file-popup\s+\.file-popup-box\s*\{[^}]*box-shadow:\s*0 16px 48px var\(--shadow-rgba-000000-30\)/,
    );
    expect(css).not.toMatch(/#file-popup\s*\{[^}]*--bg-rgba-1f2328-45/);
    expect(css).not.toMatch(/#file-popup\s+\.file-popup-box\s*\{[^}]*--shadow-rgba-1f2328-18/);
  });

  it('keeps File popup close in commands and not inside the shared control', () => {
    const popupSrc = readRel('frontend/src/file-popup/ui/popup.tsx');
    const commandSrc = readRel('frontend/src/file-popup/commands/popup.ts');
    const sharedSrc = readRel('frontend/src/shared/overlay-dismiss-button.tsx');
    expect(commandSrc).toMatch(/export function closeFilePopup/);
    expect(popupSrc).toMatch(/closeFilePopup/);
    expect(sharedSrc).not.toMatch(/closeFilePopup/);
  });
});
