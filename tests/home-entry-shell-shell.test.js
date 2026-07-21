// @vitest-environment jsdom
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

import { getBaselineEntries } from '../frontend/js/home-entry-shell/entry-config.js';
import { createContentRegistry } from '../frontend/js/home-entry-shell/content-registry.js';
import { mountHomeEntryShell } from '../frontend/js/home-entry-shell/shell.js';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '..');

/**
 * Shell UI interaction/state tests (T3).
 * Covers trigger table, OverlayChrome + content slot, vertical FAB stack, and
 * interruptible motion (logic state flips immediately).
 */

function mountFixture(extraEntries = []) {
  const config = [...getBaselineEntries(), ...extraEntries];
  const registry = createContentRegistry();
  for (const entry of config) {
    registry.register(entry.contentKey, {
      mount(slot, ctx) {
        const marker = document.createElement('div');
        marker.className = 'shell-content-marker';
        marker.dataset.entryId = ctx.entry.id;
        marker.textContent = `content:${ctx.entry.id}`;
        slot.appendChild(marker);
        return {
          unmount() {
            marker.remove();
          },
        };
      },
    });
  }
  const anchor = document.createElement('div');
  document.body.appendChild(anchor);
  const shell = mountHomeEntryShell(anchor, { config, registry, host: {} });
  return { anchor, shell, config, registry };
}

function hub(root) {
  return root.querySelector('[data-role="hub"]');
}

function entryBtn(root, entryId) {
  return root.querySelector(`[data-entry-id="${entryId}"]`);
}

function overlay(root) {
  return root.querySelector('[data-role="overlay"]');
}

function closeBtn(root) {
  return root.querySelector('[data-role="close"]');
}

function backdrop(root) {
  return root.querySelector('[data-role="backdrop"]');
}

function contentSlot(root) {
  return root.querySelector('[data-role="content-slot"]');
}

function cluster(root) {
  return root.querySelector('[data-role="cluster"]');
}

function businessEntries(root) {
  return [...root.querySelectorAll('[data-role="entry"]')];
}

function isEffectivelyHidden(el) {
  if (!el) return true;
  if (el.hidden) return true;
  if (el.getAttribute('aria-hidden') === 'true') return true;
  if (el.closest('[aria-hidden="true"]')) return true;
  return el.closest('[hidden]') != null;
}

describe('home-entry-shell shell · entry cluster + OverlayChrome + triggers (T3)', () => {
  /** @type {{ anchor: HTMLElement, shell: ReturnType<typeof mountHomeEntryShell>, config: ReturnType<typeof getBaselineEntries> } | null} */
  let fx = null;

  beforeEach(() => {
    document.body.innerHTML = '';
    fx = mountFixture();
  });

  afterEach(() => {
    fx?.shell.unmount();
    fx = null;
    document.body.innerHTML = '';
  });

  it('A: only the hub is exposed; business entries stay hidden', () => {
    const { anchor, shell } = fx;
    expect(shell.getState()).toEqual({ mode: 'A' });
    expect(hub(anchor)).not.toBeNull();
    expect(isEffectivelyHidden(hub(anchor))).toBe(false);
    for (const btn of businessEntries(anchor)) {
      expect(isEffectivelyHidden(btn)).toBe(true);
    }
    expect(overlay(anchor).hidden).toBe(true);
  });

  it('A→B: clicking hub reveals configured business entries', () => {
    const { anchor, shell, config } = fx;
    hub(anchor).click();
    expect(shell.getState()).toEqual({ mode: 'B' });
    const visible = businessEntries(anchor).filter((btn) => !isEffectivelyHidden(btn));
    expect(visible).toHaveLength(config.length);
    for (const entry of config) {
      const btn = entryBtn(anchor, entry.id);
      expect(btn).not.toBeNull();
      expect(btn.getAttribute('aria-label')).toContain(entry.title);
      expect(btn.title).toBe(entry.title);
      if (entry.fabClass) {
        expect(btn.classList.contains(entry.fabClass)).toBe(true);
      }
      if (entry.iconPaths) {
        expect(btn.querySelector('svg')).not.toBeNull();
      }
    }
  });

  it('B→C: business entry opens OverlayChrome title/close and mounts content into the unified slot', () => {
    const { anchor, shell } = fx;
    hub(anchor).click();
    entryBtn(anchor, 'read-later').click();

    expect(shell.getState()).toEqual({ mode: 'C', entryId: 'read-later' });
    expect(overlay(anchor).hidden).toBe(false);
    expect(anchor.querySelector('[data-role="title"]').textContent).toBe('Read Later');
    expect(closeBtn(anchor)).not.toBeNull();
    expect(closeBtn(anchor).getAttribute('aria-label')).toBe('Close');

    const slot = contentSlot(anchor);
    expect(slot).not.toBeNull();
    const marker = slot.querySelector('.shell-content-marker');
    expect(marker).not.toBeNull();
    expect(marker.dataset.entryId).toBe('read-later');
  });

  it('B→A dual triggers: outside cluster and re-clicking hub both collapse', () => {
    const { anchor, shell } = fx;

    hub(anchor).click();
    expect(shell.getState()).toEqual({ mode: 'B' });
    // Click outside the shell cluster (on the anchor itself, outside cluster).
    document.body.dispatchEvent(new MouseEvent('click', { bubbles: true }));
    expect(shell.getState()).toEqual({ mode: 'A' });

    hub(anchor).click();
    expect(shell.getState()).toEqual({ mode: 'B' });
    hub(anchor).click();
    expect(shell.getState()).toEqual({ mode: 'A' });
  });

  it('C→B multi-triggers: close, backdrop/outside, Escape all land on B (never A)', () => {
    const { anchor, shell } = fx;

    // close button
    hub(anchor).click();
    entryBtn(anchor, 'notes').click();
    expect(shell.getState().mode).toBe('C');
    closeBtn(anchor).click();
    expect(shell.getState()).toEqual({ mode: 'B' });

    // backdrop / outside cluster
    entryBtn(anchor, 'plan-task').click();
    expect(shell.getState().mode).toBe('C');
    backdrop(anchor).click();
    expect(shell.getState()).toEqual({ mode: 'B' });

    // Escape
    entryBtn(anchor, 'builders').click();
    expect(shell.getState().mode).toBe('C');
    document.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true }));
    expect(shell.getState()).toEqual({ mode: 'B' });
  });

  it('C keeps the entry cluster expanded; hub click while in C stays in C', () => {
    const { anchor, shell, config } = fx;
    hub(anchor).click();
    entryBtn(anchor, 'read-later').click();
    expect(shell.getState()).toEqual({ mode: 'C', entryId: 'read-later' });

    const visible = businessEntries(anchor).filter((btn) => !isEffectivelyHidden(btn));
    expect(visible.length).toBe(config.length);

    hub(anchor).click();
    expect(shell.getState()).toEqual({ mode: 'C', entryId: 'read-later' });
    expect(overlay(anchor).hidden).toBe(false);
  });

  it('B layout: bottom-right vertical FAB stack (single anchor, stacked upward)', () => {
    const { anchor } = fx;
    hub(anchor).click();

    const stack = cluster(anchor);
    expect(stack).not.toBeNull();
    expect(stack.classList.contains('home-entry-shell__cluster')).toBe(true);
    // Hub is the last child so it stays pinned at the bottom of the column stack.
    expect(stack.lastElementChild).toBe(hub(anchor));

    const css = readFileSync(join(repoRoot, 'frontend/app.css'), 'utf8');
    expect(css).toMatch(/\.home-entry-shell__cluster\s*\{[^}]*position:\s*fixed/s);
    expect(css).toMatch(/\.home-entry-shell__cluster\s*\{[^}]*right:\s*/s);
    expect(css).toMatch(/\.home-entry-shell__cluster\s*\{[^}]*bottom:\s*/s);
    expect(css).toMatch(
      /\.home-entry-shell__cluster\s*\{[^}]*(flex-direction:\s*column-reverse|flex-direction:\s*column)/s,
    );
    // Must not adopt a horizontal/fan layout as the B-state target.
    expect(css).not.toMatch(/\.home-entry-shell__cluster\s*\{[^}]*flex-direction:\s*row/s);
  });

  it('opens overlay with per-entry fixed panel size (not content-driven)', () => {
    const { anchor } = fx;
    hub(anchor).click();
    entryBtn(anchor, 'builders').click();

    const chrome = anchor.querySelector('[data-role="chrome"]');
    expect(chrome.style.getPropertyValue('--hes-panel-w')).toBe('480px');
    expect(chrome.style.getPropertyValue('--hes-panel-h')).toBe('520px');

    closeBtn(anchor).click();
    entryBtn(anchor, 'read-later').click();
    expect(chrome.style.getPropertyValue('--hes-panel-w')).toBe('340px');
    expect(chrome.style.getPropertyValue('--hes-panel-h')).toBe('460px');
  });

  it('motion is non-blocking: logic state flips immediately without waiting for animationend', () => {
    const { anchor, shell } = fx;
    const animEnd = vi.fn();
    anchor.addEventListener('animationend', animEnd);
    anchor.addEventListener('transitionend', animEnd);

    hub(anchor).click();
    expect(shell.getState()).toEqual({ mode: 'B' });
    expect(animEnd).not.toHaveBeenCalled();

    entryBtn(anchor, 'notes').click();
    expect(shell.getState()).toEqual({ mode: 'C', entryId: 'notes' });
    expect(animEnd).not.toHaveBeenCalled();

    closeBtn(anchor).click();
    expect(shell.getState()).toEqual({ mode: 'B' });
    expect(animEnd).not.toHaveBeenCalled();
  });

  it('shell source has no hardcoded business UI; entries come from config', () => {
    const shellSrc = readFileSync(
      join(repoRoot, 'frontend/js/home-entry-shell/shell.js'),
      'utf8',
    );
    expect(shellSrc).not.toMatch(/Read Later|Notes Assistant|Builders|Todos/);
    expect(shellSrc).not.toMatch(/rl-assistant-|pt-assistant-|note-assistant-|builders-modal-/);
  });

  // T7: shell-level business switch must traverse C→B→C (UI path)
  it('switching entry while in C remounts content under new entryId (C→B→C)', async () => {
    const { anchor, shell } = fx;
    hub(anchor).click();
    entryBtn(anchor, 'read-later').click();
    await Promise.resolve();
    await Promise.resolve();
    expect(shell.getState()).toEqual({ mode: 'C', entryId: 'read-later' });

    entryBtn(anchor, 'notes').click();
    await Promise.resolve();
    await Promise.resolve();
    expect(shell.getState()).toEqual({ mode: 'C', entryId: 'notes' });
    expect(contentSlot(anchor).querySelector('.shell-content-marker')?.dataset.entryId).toBe(
      'notes',
    );
  });
});
