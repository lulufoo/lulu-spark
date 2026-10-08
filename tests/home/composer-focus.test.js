// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

import {
  canFocusComposerFromDockTarget,
  focusComposerFromDock,
} from '../../frontend/src/home/commands/composer-focus.ts';

const pageSrc = readFileSync(
  join(dirname(fileURLToPath(import.meta.url)), '../../frontend/src/home/page.tsx'),
  'utf8',
);

describe('focusComposerFromDock', () => {
  it('focuses when the click is on the dock, not the textarea or a button', () => {
    const dock = document.createElement('div');
    const input = document.createElement('textarea');
    const send = document.createElement('button');
    dock.append(input, send);
    document.body.append(dock);
    let focused = false;
    input.focus = () => {
      focused = true;
    };

    const corner = document.createElement('div');
    dock.append(corner);
    expect(canFocusComposerFromDockTarget(input, dock)).toBe(true);
    expect(canFocusComposerFromDockTarget(input, corner)).toBe(true);
    expect(focusComposerFromDock(input, dock, false)).toBe(true);
    expect(focused).toBe(true);

    focused = false;
    expect(focusComposerFromDock(input, input, false)).toBe(false);
    expect(focusComposerFromDock(input, send, false)).toBe(false);
    expect(focusComposerFromDock(input, dock, true)).toBe(false);
    expect(focused).toBe(false);
    dock.remove();
  });

  it('wires the dock mouse down to focusComposerFromDock', () => {
    expect(pageSrc).toMatch(/focusComposerFromDock\(inputRef\.current, e\.target, inputLocked\)/);
  });
});

describe('home composer dock hit area', () => {
  it('lets the textarea cover the lower half; send stays overlaid', () => {
    const appCss = readFileSync(
      join(dirname(fileURLToPath(import.meta.url)), '../../frontend/app.css'),
      'utf8',
    );
    expect(appCss).toMatch(/\.home-chat-composer-dock\s*\{[^}]*position:\s*relative/);
    expect(appCss).toMatch(/\.home-chat-composer-corner\s*\{[^}]*position:\s*absolute/);
    expect(appCss).toMatch(/\.home-chat-input\s*\{[^}]*padding:\s*2px 56px 28px 0/);
    expect(pageSrc).toMatch(/input\.style\.height = '48px'/);
  });
});
