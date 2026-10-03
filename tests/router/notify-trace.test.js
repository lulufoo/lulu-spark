// @vitest-environment jsdom
import { describe, expect, it } from 'vitest';
import {
  extractTraceFromScheme,
  lastNotifyTrace,
  parseTraceId,
  rememberNotifyTrace,
} from '../../frontend/src/router/notify-trace.ts';

describe('notify-trace', () => {
  it('accepts the minted shape and rejects line breaks', () => {
    expect(parseTraceId('trace_12345678')).toBe('trace_12345678');
    expect(parseTraceId('short')).toBeNull();
    expect(parseTraceId('trace_bad\nid')).toBeNull();
  });

  it('reads trace from a workbench URL', () => {
    expect(
      extractTraceFromScheme('workbench://notes/open?id=a&path=p.md&trace=trace_12345678'),
    ).toBe('trace_12345678');
    expect(extractTraceFromScheme('workbench://read-later/list')).toBeNull();
  });

  it('remembers the last valid trace for later route.workbench', () => {
    expect(rememberNotifyTrace('trace_abcdef12')).toBe('trace_abcdef12');
    expect(lastNotifyTrace()).toBe('trace_abcdef12');
    expect(rememberNotifyTrace(null)).toBe('trace_abcdef12');
  });
});
