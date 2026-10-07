// @vitest-environment jsdom
import { afterEach, describe, expect, it, vi } from 'vitest';
import { encodePassBag, PASS_QUERY } from '../../frontend/src/auth/pass.ts';
import * as appLog from '../../frontend/src/host/app-log.ts';
import {
  businessFromSparkHost,
  extractTraceFromScheme,
  lastNotifyTrace,
  logNotifyHop,
  parseTraceId,
  rememberNotifyTrace,
} from '../../frontend/src/router/notify-trace.ts';

const HOP_ID = 'trace_12345678abcd';
const PASS = encodePassBag(HOP_ID);

describe('notify-trace', () => {
  afterEach(() => {
    vi.restoreAllMocks();
  });

  it('accepts the minted shape and rejects line breaks', () => {
    expect(parseTraceId('trace_12345678')).toBe('trace_12345678');
    expect(parseTraceId('short')).toBeNull();
    expect(parseTraceId('trace_bad\nid')).toBeNull();
  });

  it('reads trace from a spark URL', () => {
    expect(
      extractTraceFromScheme('spark://notes/open?id=a&path=p.md&trace=trace_12345678'),
    ).toBe('trace_12345678');
    expect(extractTraceFromScheme('spark://read-later/list')).toBeNull();
  });

  it('remembers the last valid trace for later route.spark', () => {
    expect(rememberNotifyTrace('trace_abcdef12')).toBe('trace_abcdef12');
    expect(lastNotifyTrace()).toBe('trace_abcdef12');
    expect(rememberNotifyTrace(null)).toBe('trace_abcdef12');
  });

  it('maps spark host to notes / read_later / login only when pass is valid', () => {
    expect(
      businessFromSparkHost(
        `spark://notes/open?id=abc&path=p.md&${PASS_QUERY}=${PASS}`,
      ),
    ).toBe('notes');
    expect(businessFromSparkHost(`spark://read-later/list?${PASS_QUERY}=${PASS}`)).toBe(
      'read_later',
    );
    expect(businessFromSparkHost(`spark://auth-login/callback?${PASS_QUERY}=${PASS}`)).toBe(
      'login',
    );
    expect(businessFromSparkHost(`spark://unknown/open?${PASS_QUERY}=${PASS}`)).toBe('app');
    expect(businessFromSparkHost('not-a-url')).toBe('app');
    expect(businessFromSparkHost('')).toBe('app');
  });

  it('does not host-map leftover missing-pass or trace-only schemes', () => {
    expect(businessFromSparkHost('spark://notes/open?id=a&path=p.md')).toBe('app');
    expect(
      businessFromSparkHost('spark://notes/open?id=a&path=p.md&trace=trace_12345678'),
    ).toBe('app');
    expect(businessFromSparkHost('spark://read-later/list')).toBe('app');
    expect(businessFromSparkHost('spark://read-later/list?trace=trace_12345678')).toBe(
      'app',
    );
    expect(businessFromSparkHost('spark://auth-login/callback')).toBe('app');
  });

  it('writes the given business and hop id and never writes os-notify', () => {
    const logSpy = vi.spyOn(appLog, 'logAppEvent').mockImplementation(() => {});
    logNotifyHop('route.to_business', HOP_ID, { outcome: 'ok' }, 'notes');
    expect(logSpy).toHaveBeenCalledWith({
      business: 'notes',
      event: 'route.to_business',
      traceId: HOP_ID,
      params: { outcome: 'ok' },
    });
    expect(logSpy.mock.calls.some(([input]) => input.business === 'os-notify')).toBe(false);
    logNotifyHop('route.to_business', null, { outcome: 'parse_fail' }, 'app');
    expect(logSpy).toHaveBeenCalledWith({
      business: 'app',
      event: 'route.to_business',
      traceId: 'trace_missing',
      params: { outcome: 'parse_fail' },
    });
  });
});
