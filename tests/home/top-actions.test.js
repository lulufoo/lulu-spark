// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createElement } from 'react';
import { describe, it, expect, vi } from 'vitest';
import { renderToHtml } from '../../frontend/src/island.ts';
import { TopActions } from '../../frontend/src/home/ui/top-actions.tsx';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

function mount(props) {
  const host = document.createElement('div');
  host.innerHTML = renderToHtml(
    createElement(TopActions, { onReadLater: vi.fn(), onSettings: vi.fn(), ...props }),
  );
  return host;
}

describe('TopActions (top bar icons)', () => {
  it('has Read Later then Settings, each with a hover name', () => {
    const host = mount({ readLaterUnread: false });
    const buttons = [...host.querySelectorAll('button')];
    expect(buttons.map((b) => b.getAttribute('data-tip'))).toEqual(['Read Later', 'Settings']);
    expect(buttons.map((b) => b.getAttribute('aria-label'))).toEqual(['Read Later', 'Settings']);
    expect(buttons[0].querySelector('[data-home-icon="read-later"]')).not.toBeNull();
    expect(buttons[1].querySelector('[data-home-icon="settings"]')).not.toBeNull();
  });

  it('marks Read Later unread only while there is something unread', () => {
    expect(mount({ readLaterUnread: true }).querySelector('[data-home-entry="read-later"]').dataset.homeUnread).toBe('true');
    expect(mount({ readLaterUnread: false }).querySelector('[data-home-entry="read-later"]').dataset.homeUnread).toBeUndefined();
  });

  it('keeps the icons out of the window drag region', () => {
    const css = readFileSync(join(repoRoot, 'frontend/app.css'), 'utf8');
    expect(css).toMatch(/\.window-drag-strip \.home-top-actions,[\s\S]*?app-region: no-drag;/);
  });
});
