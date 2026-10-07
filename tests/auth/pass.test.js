// @vitest-environment jsdom
import { describe, expect, it } from 'vitest';
import {
  decodePassBag,
  encodePassBag,
  extractPassIdFromScheme,
  newLoginTraceId,
  PASS_QUERY,
} from '../../frontend/src/auth/pass.ts';

describe('pass bag', () => {
  it('mints an id the app-log parser will accept', () => {
    const id = newLoginTraceId();
    expect(id).toMatch(/^trace_[0-9a-f]{12}$/);
    expect(decodePassBag(JSON.stringify({ id }))).toBe(id);
  });

  it('round-trips the id through a callback query', () => {
    const id = 'trace_12345678abcd';
    const url = `spark://auth-login/callback?${PASS_QUERY}=${encodePassBag(id)}#access_token=secret`;
    expect(extractPassIdFromScheme(url)).toBe(id);
  });

  it('rejects a missing or junk bag and does not read hash tokens', () => {
    expect(extractPassIdFromScheme('spark://auth-login/callback')).toBeNull();
    expect(extractPassIdFromScheme('spark://auth-login/callback?pass=not-json')).toBeNull();
    expect(
      extractPassIdFromScheme(
        'spark://auth-login/callback#access_token=trace_12345678abcd',
      ),
    ).toBeNull();
    expect(decodePassBag('{"id":"short"}')).toBeNull();
  });
});
