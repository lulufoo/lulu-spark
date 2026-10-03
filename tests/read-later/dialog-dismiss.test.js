// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { repoRoot } from '../helpers/read-frontend-js.js';

vi.mock('../../frontend/src/read-later/ui/list.tsx', () => ({
  ReadLaterList: () => null,
}));

import { ReadLaterDialog } from '../../frontend/src/read-later/ui/dialog.tsx';
import { openReadLaterDialog } from '../../frontend/src/read-later/commands/dialog.ts';
import { readLaterOpenStore } from '../../frontend/src/read-later/state/dialog-open.ts';

const GLYPH = '✕';

function visibleCopy(el) {
  return String(el?.textContent ?? '');
}

function readRel(rel) {
  return readFileSync(join(repoRoot, rel), 'utf8');
}

describe('Read Later header dismiss', () => {
  let container;
  let root;

  beforeEach(() => {
    container = document.createElement('div');
    document.body.appendChild(container);
    root = createRoot(container);
    act(() => {
      root.render(createElement(ReadLaterDialog));
    });
    act(() => {
      openReadLaterDialog();
    });
  });

  afterEach(() => {
    act(() => {
      root?.unmount();
    });
    container?.remove();
    readLaterOpenStore.set(false);
    document.body.innerHTML = '';
  });

  it('paints Read Later header dismiss as OverlayDismissButton with ✕ and no Close copy', () => {
    const src = readRel('frontend/src/read-later/ui/dialog.tsx');
    expect(src).toMatch(/<OverlayDismissButton[\s\S]*?\bid=["']btn-read-later-close["']/);
    expect(src).not.toMatch(/id="btn-read-later-close"[\s\S]*?md-header-btn/);

    const button = document.getElementById('btn-read-later-close');
    expect(button).not.toBeNull();
    expect(button.classList.contains('overlay-dismiss-button')).toBe(true);
    expect(visibleCopy(button).trim()).toBe(GLYPH);
    expect(visibleCopy(button)).not.toContain('Close');
  });

  it('keeps Read Later close on the header ✕', () => {
    const dialog = document.getElementById('read-later-dialog');
    expect(dialog.classList.contains('open')).toBe(true);
    act(() => {
      document.getElementById('btn-read-later-close').click();
    });
    expect(dialog.classList.contains('open')).toBe(false);
    expect(readLaterOpenStore.getSnapshot()).toBe(false);
  });

  it('keeps Read Later close in commands and does not add a second header x', () => {
    const dialogSrc = readRel('frontend/src/read-later/ui/dialog.tsx');
    const commandSrc = readRel('frontend/src/read-later/commands/dialog.ts');
    const sharedSrc = readRel('frontend/src/shared/overlay-dismiss-button.tsx');
    expect(commandSrc).toMatch(/export function closeReadLaterDialog/);
    expect(dialogSrc).toMatch(/closeReadLaterDialog/);
    expect(sharedSrc).not.toMatch(/closeReadLaterDialog/);

    const header = document.getElementById('read-later-dialog-header');
    expect(header.querySelectorAll('button')).toHaveLength(1);
  });

  it('does not paint per-dialog Read Later close chrome', () => {
    const css = readRel('frontend/app.css');
    expect(css).not.toMatch(/#btn-read-later-close\s*\{/);
  });
});
