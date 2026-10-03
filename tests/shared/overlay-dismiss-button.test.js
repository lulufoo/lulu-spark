// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { join } from 'node:path';
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, describe, expect, it, vi } from 'vitest';
import { OverlayDismissButton } from '../../frontend/src/shared/overlay-dismiss-button.tsx';
import { repoRoot } from '../helpers/read-frontend-js.js';

const GLYPH = '✕';

describe('OverlayDismissButton', () => {
  let container;
  let root;

  afterEach(() => {
    act(() => {
      root?.unmount();
    });
    container?.remove();
    document.body.innerHTML = '';
  });

  function render(props) {
    container = document.createElement('div');
    document.body.appendChild(container);
    root = createRoot(container);
    act(() => {
      root.render(createElement(OverlayDismissButton, props));
    });
    return container.querySelector('button');
  }

  function visibleCopy(button) {
    return String(button?.textContent ?? '');
  }

  function accessibleName(button) {
    return button.getAttribute('aria-label') || button.getAttribute('title') || visibleCopy(button).trim();
  }

  it('renders the Settings glyph and no visible Close copy by default', () => {
    const button = render();
    expect(button).not.toBeNull();
    expect(visibleCopy(button).trim()).toBe(GLYPH);
    expect(visibleCopy(button)).not.toContain('Close');
  });

  it('keeps caller id and exposes accessible name Close without visible Close copy', () => {
    const button = render({ id: 'btn-demo-close', title: 'Close' });
    expect(button.id).toBe('btn-demo-close');
    expect(accessibleName(button)).toBe('Close');
    expect(visibleCopy(button).trim()).toBe(GLYPH);
    expect(visibleCopy(button)).not.toContain('Close');
  });

  it('forwards onClick once and does not close or mutate overlay state itself', () => {
    const overlay = document.createElement('div');
    overlay.id = 'demo-overlay';
    overlay.className = 'open';
    document.body.appendChild(overlay);
    const dialogState = { open: true };
    const onClick = vi.fn();

    const button = render({ onClick });
    act(() => {
      button.click();
    });

    expect(onClick).toHaveBeenCalledTimes(1);
    expect(overlay.classList.contains('open')).toBe(true);
    expect(dialogState.open).toBe(true);
    expect(document.getElementById('demo-overlay')).toBe(overlay);
  });

  it('does not call onClick when disabled', () => {
    const onClick = vi.fn();
    const button = render({ disabled: true, onClick });
    expect(button.disabled).toBe(true);
    act(() => {
      button.click();
    });
    expect(onClick).not.toHaveBeenCalled();
  });

  it('renders the glyph when onClick is omitted and does not throw on click', () => {
    let button;
    expect(() => {
      button = render();
      act(() => {
        button.click();
      });
    }).not.toThrow();
    expect(visibleCopy(button).trim()).toBe(GLYPH);
  });

  it('mounts with an empty id and does not fabricate a close handler', () => {
    const overlay = document.createElement('div');
    overlay.className = 'open';
    document.body.appendChild(overlay);
    const button = render({ id: '' });
    expect(button).not.toBeNull();
    expect(button.getAttribute('id') ?? '').toBe('');
    act(() => {
      button.click();
    });
    expect(overlay.classList.contains('open')).toBe(true);
  });

  it('app.css defines Settings-style borderless chrome for the shared dismiss class', () => {
    const css = readFileSync(join(repoRoot, 'frontend/app.css'), 'utf8');
    expect(css).toMatch(/\.overlay-dismiss-button\s*\{[^}]*border:\s*none/s);
    expect(css).toMatch(/\.overlay-dismiss-button\s*\{[^}]*background:\s*none/s);
  });
});
