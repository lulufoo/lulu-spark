// @vitest-environment jsdom
import { describe, expect, it } from 'vitest';

import { formatMessageWhen, hydrateTurns } from '../../frontend/src/home/state/store.ts';

describe('formatMessageWhen', () => {
  it('formats unix seconds as M/D, HH:mm', () => {
    const ts = Math.floor(new Date(2026, 9, 2, 20, 0).getTime() / 1000);
    expect(formatMessageWhen(ts)).toBe('10/2, 20:00');
  });

  it('returns empty for missing or invalid values', () => {
    expect(formatMessageWhen(undefined)).toBe('');
    expect(formatMessageWhen(0)).toBe('');
    expect(formatMessageWhen('nope')).toBe('');
  });
});

describe('hydrateTurns', () => {
  it('keeps created_at as createdAt', () => {
    expect(
      hydrateTurns([
        { role: 'user', content: 'hi', created_at: 1760000000 },
        { role: 'assistant', content: 'yo' },
      ]),
    ).toEqual([
      { role: 'user', text: 'hi', createdAt: 1760000000 },
      { role: 'assistant', text: 'yo' },
    ]);
  });

  it('drops non-positive created_at', () => {
    expect(hydrateTurns([{ role: 'user', content: 'hi', created_at: 0 }])).toEqual([
      { role: 'user', text: 'hi' },
    ]);
  });
});
