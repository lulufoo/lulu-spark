// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';
import { repoRoot } from '../helpers/read-frontend-js.js';

vi.mock('../../frontend/src/host/api.ts', () => ({
  invoke: vi.fn(),
}));

import { BindDialog } from '../../frontend/src/app-shell/ui/bind-dialog.tsx';
import { bindOpenStore } from '../../frontend/src/app-shell/commands/bind-dialog.ts';

const GLYPH = '✕';

function visibleCopy(el) {
  return String(el?.textContent ?? '');
}

function readRel(rel) {
  return readFileSync(`${repoRoot}/${rel}`, 'utf8');
}

describe('Bind header dismiss', () => {
  let container;
  let root;

  beforeEach(() => {
    container = document.createElement('div');
    document.body.appendChild(container);
    root = createRoot(container);
    act(() => {
      root.render(createElement(BindDialog));
    });
    act(() => {
      bindOpenStore.set(true);
    });
  });

  afterEach(() => {
    act(() => {
      root?.unmount();
    });
    container?.remove();
    bindOpenStore.set(false);
    document.body.innerHTML = '';
  });

  it('paints Bind header dismiss as OverlayDismissButton with ✕ and no Close copy', () => {
    const src = readRel('frontend/src/app-shell/ui/bind-dialog.tsx');
    expect(src).toMatch(/<OverlayDismissButton[\s\S]*?\bid=["']btn-bind-close["']/);

    const button = document.getElementById('btn-bind-close');
    expect(button).not.toBeNull();
    expect(button.classList.contains('overlay-dismiss-button')).toBe(true);
    expect(visibleCopy(button).trim()).toBe(GLYPH);
    expect(visibleCopy(button)).not.toContain('Close');
  });

  it('keeps Bind close on the header ✕', () => {
    const dialog = document.getElementById('bind-dialog');
    expect(dialog.classList.contains('open')).toBe(true);
    act(() => {
      document.getElementById('btn-bind-close').click();
    });
    expect(dialog.classList.contains('open')).toBe(false);
    expect(bindOpenStore.getSnapshot()).toBe(false);
  });

  it('keeps Bind close in commands and does not add a second header x', () => {
    const dialogSrc = readRel('frontend/src/app-shell/ui/bind-dialog.tsx');
    const commandSrc = readRel('frontend/src/app-shell/commands/bind-dialog.ts');
    const sharedSrc = readRel('frontend/src/shared/overlay-dismiss-button.tsx');
    expect(commandSrc).toMatch(/export function closeBindDialog/);
    expect(dialogSrc).toMatch(/closeBindDialog/);
    expect(sharedSrc).not.toMatch(/closeBindDialog/);

    const header = document.getElementById('bind-dialog-header');
    expect(header.querySelectorAll('button')).toHaveLength(1);
    const footer = document.getElementById('bind-dialog-footer');
    expect(visibleCopy(footer)).toMatch(/A new code is available/);
    expect(visibleCopy(document.getElementById('btn-bind-refresh'))).toMatch(/New code/);
    expect(visibleCopy(footer)).not.toMatch(/Close/);
  });
});
