// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { sidebarCollapsedStore } from '../../frontend/src/app-shell/state/sidebar-collapsed.ts';
import { SidebarToggle } from '../../frontend/src/app-shell/ui/sidebar-toggle.tsx';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

describe('SidebarToggle', () => {
  let container;
  let root;

  afterEach(() => {
    act(() => {
      root?.unmount();
    });
    container?.remove();
    document.body.innerHTML = '';
    sidebarCollapsedStore.set(false);
  });

  function render(props) {
    container = document.createElement('div');
    document.body.appendChild(container);
    root = createRoot(container);
    act(() => {
      root.render(createElement(SidebarToggle, {
        collapsed: false,
        controlsId: 'home-nav',
        onToggle: vi.fn(),
        ...props,
      }));
    });
    return container.querySelector('button');
  }

  it('names the expanded sidebar Hide sidebar and points at the nav', () => {
    const button = render({ collapsed: false, controlsId: 'notes-nav' });
    expect(button.type).toBe('button');
    expect(button.getAttribute('data-tip')).toBe('Hide sidebar');
    expect(button.getAttribute('aria-label')).toBe('Hide sidebar');
    expect(button.getAttribute('aria-expanded')).toBe('true');
    expect(button.getAttribute('aria-controls')).toBe('notes-nav');
    expect(button.hasAttribute('title')).toBe(false);
    expect(button.querySelector('rect')?.getAttribute('x')).toBe('3');
    expect(button.querySelector('path')?.getAttribute('d')).toBe('M9 4v16');
  });

  it('names the collapsed sidebar Show sidebar', () => {
    const button = render({ collapsed: true, controlsId: 'home-nav' });
    expect(button.getAttribute('data-tip')).toBe('Show sidebar');
    expect(button.getAttribute('aria-label')).toBe('Show sidebar');
    expect(button.getAttribute('aria-expanded')).toBe('false');
    expect(button.getAttribute('aria-controls')).toBe('home-nav');
  });

  it('forwards one click and does not change the shared flag itself', () => {
    sidebarCollapsedStore.set(false);
    const onToggle = vi.fn();
    const button = render({ collapsed: false, onToggle });
    act(() => {
      button.click();
    });
    expect(onToggle).toHaveBeenCalledTimes(1);
    expect(sidebarCollapsedStore.getSnapshot()).toBe(false);
  });

  it('does not import collapse state or commands', () => {
    const source = readFileSync(
      join(repoRoot, 'frontend/src/app-shell/ui/sidebar-toggle.tsx'),
      'utf8',
    );
    expect(source).not.toMatch(/sidebar-collapsed|app-shell\/state|app-shell\/commands/);
  });

  it('uses the existing 28px tool target, one hover, and stays out of the drag region', () => {
    const css = readFileSync(join(repoRoot, 'frontend/app.css'), 'utf8');
    expect(css).toMatch(/\.sidebar-toggle\s*\{[^}]*width:\s*28px;/);
    expect(css).toMatch(/\.sidebar-toggle\s*\{[^}]*height:\s*28px;/);
    expect(css).toMatch(/\.sidebar-toggle-icon\s*\{[^}]*width:\s*18px;/);
    expect(css).toMatch(/\.sidebar-toggle-icon\s*\{[^}]*height:\s*18px;/);
    expect(css).toMatch(/\.sidebar-toggle\s*\{[^}]*app-region:\s*no-drag;/);
    expect(css).toMatch(/\.home-top-action:focus-visible,\s*\.sidebar-toggle:focus-visible\s*\{[^}]*outline:\s*2px solid var\(--border-accent\)/);
    const hoverRules = css.match(/\.sidebar-toggle:hover\s*\{/g) ?? [];
    expect(hoverRules).toHaveLength(1);
  });
});
