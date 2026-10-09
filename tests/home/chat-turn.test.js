// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const when = Math.floor(new Date(2026, 9, 2, 20, 0).getTime() / 1000);

describe('chat turn · source', () => {
  it('wires ChatTurn from Home and keeps copy in commands', () => {
    const page = readFileSync(join(repoRoot, 'frontend/src/home/page.tsx'), 'utf8');
    expect(page).toMatch(/from ['"]\.\/ui\/chat-turn\.tsx['"]/);
    expect(page).toMatch(/from ['"]\.\/commands\/copy-message\.ts['"]/);
    expect(page).toMatch(/<ChatTurn/);
    expect(page).toMatch(/copyMessageText/);
  });

  it('hides user time and copy until hover, keeps assistant copy visible', () => {
    const css = readFileSync(join(repoRoot, 'frontend/app.css'), 'utf8');
    expect(css).toMatch(/\.home-chat-turn-meta\s*\{/);
    expect(css).toMatch(
      /\.home-chat-turn--user \.home-chat-turn-time,\s*\.home-chat-turn--user \.home-chat-turn-copy\s*\{[^}]*opacity:\s*0/,
    );
    expect(css).toMatch(
      /\.home-chat-turn--user:hover \.home-chat-turn-time,\s*\.home-chat-turn--user:hover \.home-chat-turn-copy/,
    );
    expect(css).toMatch(
      /\.home-chat-turn--assistant \.home-chat-turn-time,\s*\.home-chat-turn--error \.home-chat-turn-time\s*\{[^}]*opacity:\s*0/,
    );
    expect(css).toMatch(
      /\.home-chat-turn--assistant:hover \.home-chat-turn-time,\s*\.home-chat-turn--assistant:focus-within \.home-chat-turn-time/,
    );
  });
});

describe('ChatTurn', () => {
  let container;
  let root;
  let ChatTurn;

  beforeEach(async () => {
    ({ ChatTurn } = await import('../../frontend/src/home/ui/chat-turn.tsx'));
    container = document.createElement('div');
    document.body.appendChild(container);
    root = createRoot(container);
  });

  afterEach(() => {
    act(() => {
      root?.unmount();
    });
    container?.remove();
  });

  function render(props, child = 'hello') {
    act(() => {
      root.render(createElement(ChatTurn, props, child));
    });
  }

  it('shows time and copy on a user turn', () => {
    render({ kind: 'user', createdAt: when, onCopy: vi.fn(async () => true) });
    const turn = container.querySelector('.home-chat-turn--user');
    expect(turn?.querySelector('.home-chat-bubble--user')?.textContent).toBe('hello');
    expect(turn?.querySelector('.home-chat-turn-time')?.textContent).toBe('10/2, 20:00');
    expect(turn?.querySelector('[data-role="copy-message"]')?.getAttribute('aria-label')).toBe(
      'Copy',
    );
  });

  it('keeps assistant copy in the row and places time after it', () => {
    render({ kind: 'assistant', createdAt: when, onCopy: vi.fn(async () => true) }, 'reply');
    const meta = container.querySelector('.home-chat-turn--assistant .home-chat-turn-meta');
    const copy = meta?.querySelector('[data-role="copy-message"]');
    const time = meta?.querySelector('.home-chat-turn-time');
    expect(copy).not.toBeNull();
    expect(time?.textContent).toBe('10/2, 20:00');
    expect(meta?.innerHTML.indexOf('copy-message')).toBeLessThan(meta?.innerHTML.indexOf('home-chat-turn-time'));
  });

  it('copies through onCopy and flashes Copied', async () => {
    const onCopy = vi.fn(async () => true);
    render({ kind: 'assistant', createdAt: when, onCopy });
    act(() => {
      container.querySelector('[data-role="copy-message"]').click();
    });
    await act(async () => {
      await Promise.resolve();
    });
    expect(onCopy).toHaveBeenCalledTimes(1);
    expect(container.querySelector('[data-role="copy-message"]')?.getAttribute('aria-label')).toBe(
      'Copied',
    );
  });
});
