// @vitest-environment jsdom
import { describe, expect, it } from 'vitest';
import { formatCommittedAt, paintKbCommittedAt } from '../../frontend/src/knowledge/state/committed-at.ts';

describe('formatCommittedAt', () => {
  it('formats unix seconds as local YYYY-MM-DD HH:mm', () => {
    const unix = Date.UTC(2026, 8, 6, 15, 40) / 1000;
    const expected = formatCommittedAt(unix);
    const date = new Date(unix * 1000);
    const pad = (n) => String(n).padStart(2, '0');
    expect(expected).toBe(
      `${date.getFullYear()}-${pad(date.getMonth() + 1)}-${pad(date.getDate())} ${pad(date.getHours())}:${pad(date.getMinutes())}`,
    );
  });

  it('returns empty for missing values', () => {
    expect(formatCommittedAt(null)).toBe('');
    expect(formatCommittedAt(0)).toBe('');
    expect(formatCommittedAt('1700000000')).toBe('');
  });
});

describe('paintKbCommittedAt', () => {
  it('shows and hides the committed-at span', () => {
    const el = document.createElement('span');
    paintKbCommittedAt(el, 1_704_067_200);
    expect(el.hidden).toBe(false);
    expect(el.textContent).toMatch(/^\d{4}-\d{2}-\d{2} \d{2}:\d{2}$/);
    paintKbCommittedAt(el, null);
    expect(el.hidden).toBe(true);
    expect(el.textContent).toBe('');
  });
});
