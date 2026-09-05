import { describe, expect, it } from 'vitest';

import { composerInputLocked, composerLocked } from '../../frontend/src/home/state/store.ts';

function snap(overrides = {}) {
  return {
    sessions: [],
    currentSessionId: 's1',
    messages: [],
    staged: [],
    hostBound: true,
    progressByChat: {},
    inFlightIds: [],
    channelUnread: { notes: false, read_later: false, todos: false },
    ...overrides,
  };
}

describe('home composer locks', () => {
  it('unbound locks both input and send', () => {
    const state = snap({ hostBound: false });
    expect(composerInputLocked(state)).toBe(true);
    expect(composerLocked(state)).toBe(true);
  });

  it('in-flight locks send only', () => {
    const state = snap({ inFlightIds: ['s1'] });
    expect(composerInputLocked(state)).toBe(false);
    expect(composerLocked(state)).toBe(true);
  });

  it('idle bound locks neither', () => {
    const state = snap();
    expect(composerInputLocked(state)).toBe(false);
    expect(composerLocked(state)).toBe(false);
  });
});
