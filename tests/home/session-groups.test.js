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
  it('gives Today and Yesterday their names and every other day its date, newest first', () => {
    const groups = groupSessionsByDay(
      [
        at('a', secs(2026, 10, 18, 9)),
        at('b', secs(2026, 10, 17, 22)),
        at('c', secs(2026, 10, 13, 12)),
        at('d', secs(2026, 10, 13, 8)),
      ],
      NOW,
    );
    expect(groups.map((g) => [g.key, g.label, idsOf(g)])).toEqual([
      ['today', 'Today', ['a']],
      ['yesterday', 'Yesterday', ['b']],
      ['2026-10-13', 'Oct 13', ['c', 'd']],
    ]);
  });

  it('adds the year to a date heading only when it is not the current year', () => {
    const groups = groupSessionsByDay(
      [at('a', secs(2026, 1, 2, 9)), at('b', secs(2025, 12, 31, 9))],
      NOW,
    );
    expect(groups.map((g) => g.label)).toEqual(['Jan 2', 'Dec 31, 2025']);
  });

  it('has no Earlier group: a chat from long ago gets its own date', () => {
    const groups = groupSessionsByDay([at('a', secs(2026, 3, 1, 9))], NOW);
    expect(groups.map((g) => g.label)).toEqual(['Mar 1']);
  });

  it('puts the day boundaries on local midnight', () => {
    const groups = groupSessionsByDay(
      [
        at('todayStart', secs(2026, 10, 18, 0, 0, 0)),
        at('yesterdayEnd', secs(2026, 10, 17, 23, 59, 59)),
        at('yesterdayStart', secs(2026, 10, 17, 0, 0, 0)),
        at('dayBeforeEnd', secs(2026, 10, 16, 23, 59, 59)),
      ],
      NOW,
    );
    expect(groups.map((g) => [g.label, idsOf(g)])).toEqual([
      ['Today', ['todayStart']],
      ['Yesterday', ['yesterdayEnd', 'yesterdayStart']],
      ['Oct 16', ['dayBeforeEnd']],
    ]);
  });

  it('leaves out a day that has no chats', () => {
    const groups = groupSessionsByDay([at('a', secs(2026, 10, 18, 9)), at('c', secs(2026, 10, 1))], NOW);
    expect(groups.map((g) => g.key)).toEqual(['today', '2026-10-01']);
    expect(groupSessionsByDay([], NOW)).toEqual([]);
  });

  it('puts a chat with a missing or invalid updated_at last, under No date', () => {
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
    expect(groups.map((g) => [g.key, g.label, idsOf(g)])).toEqual([
      ['today', 'Today', ['today']],
      ['undated', 'No date', ['none', 'zero', 'nan', 'absent']],
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
        at('c', secs(2026, 10, 13, 9)),
        at('d', secs(2026, 10, 13, 8)),
      ]);
      const groups = [...host.querySelectorAll('.home-chat-session-group')];
      expect(groups.map((g) => g.querySelector('.home-chat-session-group-label').textContent)).toEqual([
        'Today',
        'Yesterday',
        'Oct 13',
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

  it('tints a hovered chat row with the hover colour and the open chat with the selected colour, no white card', () => {
    const css = readFileSync(join(repoRoot, 'frontend/home.css'), 'utf8');
    const hover = css.match(/\.home-chat-session:hover \{[^}]*\}/)?.[0] ?? '';
    const active = css.match(/\.home-chat-session\.is-active \{[^}]*\}/)?.[0] ?? '';
    expect(hover).toMatch(/background: var\(--bg-hover\)/);
    expect(hover).toMatch(/box-shadow: none/);
    expect(active).toMatch(/background: var\(--bg-selected\)/);
    expect(active).toMatch(/box-shadow: none/);
    expect(css).not.toMatch(/\.home-chat-session\.is-active \{[^}]*var\(--bg-surface\)/);
  });

  it('leaves a gap between chat rows, so the active row and a hovered row do not touch', () => {
    const css = readFileSync(join(repoRoot, 'frontend/home.css'), 'utf8');
    const rule = css.match(/\.home-chat-session \+ \.home-chat-session \{[^}]*\}/)?.[0] ?? '';
    expect(rule).toMatch(/margin-top: 4px/);
  });

  it('keeps the delete button in the layout, only hidden, so a title is cut at the same place hovered or not', () => {
    const css = readFileSync(join(repoRoot, 'frontend/home.css'), 'utf8');
    const idle = css.match(/\.home-chat-session-delete \{[^}]*\}/)?.[0] ?? '';
    expect(idle).toMatch(/display: inline-flex/);
    expect(idle).toMatch(/opacity: 0/);
    expect(idle).toMatch(/pointer-events: none/);
    expect(idle).not.toMatch(/display: none/);
    const shown = css.match(/\.home-chat-session:hover \.home-chat-session-delete,\s*\.home-chat-session:focus-within \.home-chat-session-delete \{[^}]*\}/)?.[0] ?? '';
    expect(shown).toMatch(/opacity: 1/);
    expect(shown).toMatch(/pointer-events: auto/);
  });

  it('stretches the open button to the full row height, so the whole row is clickable', () => {
    const css = readFileSync(join(repoRoot, 'frontend/home.css'), 'utf8');
    const rule = css.match(/\.home-chat-session-open \{[^}]*\}/)?.[0] ?? '';
    expect(rule).toMatch(/align-self: stretch/);
  });
});
