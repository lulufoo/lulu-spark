// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { repoRoot } from '../helpers/read-frontend-js.js';
import { SkillsDialog } from '../../frontend/src/app-shell/ui/skills-dialog.tsx';
import { _openSkillsDialog, skillsStore } from '../../frontend/src/app-shell/commands/skills-dialog.ts';

const GLYPH = '✕';

function visibleCopy(el) {
  return String(el?.textContent ?? '');
}

function readRel(rel) {
  return readFileSync(`${repoRoot}/${rel}`, 'utf8');
}

describe('Skills header dismiss', () => {
  let container;
  let root;

  beforeEach(() => {
    container = document.createElement('div');
    document.body.appendChild(container);
    root = createRoot(container);
    act(() => {
      root.render(createElement(SkillsDialog));
    });
    act(() => {
      _openSkillsDialog();
    });
  });

  afterEach(() => {
    act(() => {
      root?.unmount();
    });
    container?.remove();
    skillsStore.set({ open: false, title: '', items: [] });
    document.body.innerHTML = '';
  });

  it('paints Skills header dismiss as OverlayDismissButton with ✕ and no Close copy', () => {
    const src = readRel('frontend/src/app-shell/ui/skills-dialog.tsx');
    expect(src).toMatch(/<OverlayDismissButton[\s\S]*?\bid=["']btn-skills-dialog-close["']/);

    const button = document.getElementById('btn-skills-dialog-close');
    expect(button).not.toBeNull();
    expect(button.classList.contains('overlay-dismiss-button')).toBe(true);
    expect(visibleCopy(button).trim()).toBe(GLYPH);
    expect(visibleCopy(button)).not.toContain('Close');
  });

  it('keeps Skills close on the header ✕', () => {
    const dialog = document.getElementById('skills-dialog');
    expect(dialog.classList.contains('open')).toBe(true);
    act(() => {
      document.getElementById('btn-skills-dialog-close').click();
    });
    expect(dialog.classList.contains('open')).toBe(false);
    expect(skillsStore.getSnapshot().open).toBe(false);
  });

  it('keeps Skills close in commands and does not add a second header x', () => {
    const dialogSrc = readRel('frontend/src/app-shell/ui/skills-dialog.tsx');
    const commandSrc = readRel('frontend/src/app-shell/commands/skills-dialog.ts');
    const sharedSrc = readRel('frontend/src/shared/overlay-dismiss-button.tsx');
    expect(commandSrc).toMatch(/export function _closeSkillsDialog/);
    expect(dialogSrc).toMatch(/_closeSkillsDialog/);
    expect(sharedSrc).not.toMatch(/_closeSkillsDialog/);

    const header = document.getElementById('skills-dialog-header');
    expect(header.querySelectorAll('button')).toHaveLength(1);
    expect(document.getElementById('skills-dialog-body').querySelector('button')).toBeNull();
  });
});
