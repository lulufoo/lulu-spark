// @vitest-environment jsdom
import { beforeEach, describe, expect, it } from 'vitest';
import {
  beginAuthLoginWait,
  consumeMatchingAuthLoginPass,
} from '../../frontend/src/auth/login-wait.ts';

function clearWait() {
  beginAuthLoginWait('trace_clearwait01');
  consumeMatchingAuthLoginPass('trace_clearwait01');
}

beforeEach(clearWait);

describe('beginAuthLoginWait / consumeMatchingAuthLoginPass', () => {
  it('consumes only the matching in-flight id', () => {
    beginAuthLoginWait('trace_waitaaaaaa');
    expect(consumeMatchingAuthLoginPass('trace_waitbbbbbb')).toBe(false);
    expect(consumeMatchingAuthLoginPass('trace_waitaaaaaa')).toBe(true);
    expect(consumeMatchingAuthLoginPass('trace_waitaaaaaa')).toBe(false);
  });

  it('does not consume on null or when nothing is waiting', () => {
    expect(consumeMatchingAuthLoginPass(null)).toBe(false);
    expect(consumeMatchingAuthLoginPass('trace_waitaaaaaa')).toBe(false);
    beginAuthLoginWait('trace_waitaaaaaa');
    expect(consumeMatchingAuthLoginPass(null)).toBe(false);
    expect(consumeMatchingAuthLoginPass('trace_waitaaaaaa')).toBe(true);
  });

  it('replaces the previous wait on a later begin', () => {
    beginAuthLoginWait('trace_waitfirst01');
    beginAuthLoginWait('trace_waitsecond2');
    expect(consumeMatchingAuthLoginPass('trace_waitfirst01')).toBe(false);
    expect(consumeMatchingAuthLoginPass('trace_waitsecond2')).toBe(true);
  });
});
