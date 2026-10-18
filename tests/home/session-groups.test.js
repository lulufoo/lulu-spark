// @vitest-environment jsdom
import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { createElement } from 'react';
import { describe, expect, it, vi } from 'vitest';
import { renderToHtml } from '../../frontend/src/island.ts';
import { groupSessionsByDay } from '../../frontend/src/home/state/store.ts';
import { SessionList } from '../../frontend/src/home/ui/session-list.tsx';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

// Built from local calendar parts, so the cases hold in any time zone.
const secs = (y, mo, d, h = 0, mi = 0, s = 0) => new Date(y, mo - 1, d, h, mi, s).getTime() / 1000;
const NOW = secs(2026, 10, 18, 15, 30);
const at = (id, updated_at) => ({ session_id: id, title: id, updated_at });
const idsOf = (group) => group.sessions.map((s) => s.session_id);

describe('groupSessionsByDay', () => {
  it('splits by local calendar day into Today / Yesterday / Earlier, in that order', () => {
    const groups = groupSessionsByDay(
      [at('a', secs(2026, 10, 18, 9)), at('b', secs(2026, 10, 17, 22)), at('c', secs(2026, 10, 3, 12))],
      NOW,
    );
    expect(groups.map((g) => [g.key, g.label, idsOf(g)])).toEqual([
      ['today', 'Today', ['a']],
      ['yesterday', 'Yesterday', ['b']],
      ['earlier', 'Earlier', ['c']],
    ]);
  });

  it('puts the day boundaries on local midnight', () => {
    const groups = groupSessionsByDay(
      [
        at('todayStart', secs(2026, 10, 18, 0, 0, 0)),
        at('yesterdayEnd', secs(2026, 10, 17, 23, 59, 59)),
        at('yesterdayStart', secs(2026, 10, 17, 0, 0, 0)),
        at('earlierEnd', secs(2026, 10, 16, 23, 59, 59)),
      ],
      NOW,
    );
    expect(groups.map((g) => [g.key, idsOf(g)])).toEqual([
      ['today', ['todayStart']],
      ['yesterday', ['yesterdayEnd', 'yesterdayStart']],
      ['earlier', ['earlierEnd']],
    ]);
  });

  it('leaves out a group that has no chats', () => {
    const groups = groupSessionsByDay([at('a', secs(2026, 10, 18, 9)), at('c', secs(2026, 9, 1))], NOW);
    expect(groups.map((g) => g.key)).toEqual(['today', 'earlier']);
    expect(groupSessionsByDay([], NOW)).toEqual([]);
  });

  it('puts a chat with a missing or invalid updated_at into Earlier', () => {
    const groups = groupSessionsByDay(
      [
        at('none', undefined),
        at('zero', 0),
        at('nan', 'soon'),
        { session_id: 'absent', title: 'absent' },
        at('today', secs(2026, 10, 18, 8)),
      ],
      NOW,
    );
    expect(groups.map((g) => [g.key, idsOf(g)])).toEqual([
      ['today', ['today']],
      ['earlier', ['none', 'zero', 'nan', 'absent']],
    ]);
  });

  it('keeps the input order inside a group', () => {
    const groups = groupSessionsByDay(
      [at('late', secs(2026, 10, 18, 14)), at('mid', secs(2026, 10, 18, 10)), at('early', secs(2026, 10, 18, 1))],
      NOW,
    );
    expect(idsOf(groups[0])).toEqual(['late', 'mid', 'early']);
  });

  it('reads "now" from the clock when it is not passed', () => {
    vi.useFakeTimers();
    try {
      vi.setSystemTime(new Date(2026, 9, 18, 15, 30));
      const groups = groupSessionsByDay([at('a', secs(2026, 10, 18, 9)), at('b', secs(2026, 10, 17, 9))]);
      expect(groups.map((g) => g.key)).toEqual(['today', 'yesterday']);
    } finally {
      vi.useRealTimers();
    }
  });
});

describe('SessionList headings', () => {
  function mount(sessions) {
    const host = document.createElement('div');
    host.innerHTML = renderToHtml(
      createElement(SessionList, {
        sessions,
        currentSessionId: '',
        progressByChat: {},
        inFlightIds: [],
        onSelect: vi.fn(),
        onDelete: vi.fn(),
      }),
    );
    return host;
  }

  it('renders a heading per non-empty group, in order, with its rows under it', () => {
    vi.useFakeTimers();
    try {
      vi.setSystemTime(new Date(2026, 9, 18, 15, 30));
      const host = mount([
        at('a', secs(2026, 10, 18, 9)),
        at('b', secs(2026, 10, 17, 9)),
        at('c', secs(2026, 10, 1, 9)),
        at('d', secs(2026, 9, 1, 9)),
      ]);
      const groups = [...host.querySelectorAll('.home-chat-session-group')];
      expect(groups.map((g) => g.querySelector('.home-chat-session-group-label').textContent)).toEqual([
        'Today',
        'Yesterday',
        'Earlier',
      ]);
      expect(groups.map((g) => [...g.querySelectorAll('.home-chat-session-label')].map((el) => el.textContent))).toEqual([
        ['a'],
        ['b'],
        ['c', 'd'],
      ]);
    } finally {
      vi.useRealTimers();
    }
  });

  it('shows only the headings that have chats', () => {
    vi.useFakeTimers();
    try {
      vi.setSystemTime(new Date(2026, 9, 18, 15, 30));
      const host = mount([at('a', secs(2026, 10, 18, 9))]);
      expect([...host.querySelectorAll('.home-chat-session-group-label')].map((el) => el.textContent)).toEqual(['Today']);
    } finally {
      vi.useRealTimers();
    }
  });

  it('keeps the empty state and exposes each group to assistive tech', () => {
    expect(mount([]).querySelector('.home-chat-sessions-empty')?.textContent).toBe('No conversations yet.');
    const host = mount([at('a', secs(2026, 10, 18, 9))]);
    const group = host.querySelector('.home-chat-session-group');
    expect(group.getAttribute('role')).toBe('group');
    expect(group.getAttribute('aria-label')).toBe('Today');
    expect(group.querySelector('.home-chat-session-group-label').getAttribute('aria-hidden')).toBe('true');
  });

  it('styles the heading like the other sidebar labels', () => {
    const css = readFileSync(join(repoRoot, 'frontend/home.css'), 'utf8');
    const rule = css.match(/\.home-chat-session-group-label \{[^}]*\}/)?.[0] ?? '';
    expect(rule).toMatch(/font-size: 11px/);
    expect(rule).toMatch(/color: var\(--fg-subtle\)/);
  });

  it('keeps the chat row text close to the right edge, so a long title is not cut short of it', () => {
    const css = readFileSync(join(repoRoot, 'frontend/home.css'), 'utf8');
    const rule = css.match(/\.home-chat-session-open \{[^}]*\}/)?.[0] ?? '';
    expect(rule).toMatch(/padding: 0 2px 0 8px/);
  });

  it('stretches the open button to the full row height, so the whole row is clickable', () => {
    const css = readFileSync(join(repoRoot, 'frontend/home.css'), 'utf8');
    const rule = css.match(/\.home-chat-session-open \{[^}]*\}/)?.[0] ?? '';
    expect(rule).toMatch(/align-self: stretch/);
  });
});
