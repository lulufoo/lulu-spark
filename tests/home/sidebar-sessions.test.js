// @vitest-environment node
import { describe, expect, it } from 'vitest';
import { SIDEBAR_CHAT_LIMIT, sidebarSessions } from '../../frontend/src/home/state/sidebar-sessions.ts';

const sessionsOf = (count) => Array.from({ length: count }, (_, i) => ({ session_id: `s${i + 1}` }));
const idsOf = (list) => list.map((s) => s.session_id);

describe('sidebarSessions', () => {
  it('lists the newest 3', () => {
    expect(SIDEBAR_CHAT_LIMIT).toBe(3);
    expect(idsOf(sidebarSessions(sessionsOf(10), ''))).toEqual(['s1', 's2', 's3']);
  });

  it('lists everything when there are 3 or fewer', () => {
    expect(idsOf(sidebarSessions(sessionsOf(2), 's2'))).toEqual(['s1', 's2']);
    expect(idsOf(sidebarSessions(sessionsOf(3), 's3'))).toEqual(['s1', 's2', 's3']);
  });

  it('keeps the list as is when the current chat is among the newest 3', () => {
    expect(idsOf(sidebarSessions(sessionsOf(10), 's2'))).toEqual(['s1', 's2', 's3']);
  });

  it('gives an older current chat the last slot, so it never drops out', () => {
    expect(idsOf(sidebarSessions(sessionsOf(10), 's8'))).toEqual(['s1', 's2', 's8']);
  });

  it('ignores a current id that is not in the list', () => {
    expect(idsOf(sidebarSessions(sessionsOf(10), 'gone'))).toEqual(['s1', 's2', 's3']);
  });
});
