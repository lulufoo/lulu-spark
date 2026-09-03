import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

import { createImeEnterGuard, isImeKey } from '../../frontend/src/home/ime-enter.ts';

describe('isImeKey', () => {
  it('treats isComposing as IME', () => {
    expect(isImeKey({ isComposing: true, keyCode: 13 })).toBe(true);
  });

  it('treats keyCode 229 as IME process', () => {
    expect(isImeKey({ isComposing: false, keyCode: 229 })).toBe(true);
  });

  it('allows a plain Enter', () => {
    expect(isImeKey({ isComposing: false, keyCode: 13 })).toBe(false);
  });
});

describe('createImeEnterGuard', () => {
  /** @type {ReturnType<typeof createImeEnterGuard>} */
  let guard;

  beforeEach(() => {
    vi.useFakeTimers();
    guard = createImeEnterGuard();
  });

  afterEach(() => {
    guard.dispose();
    vi.useRealTimers();
  });

  it('blocks Enter while composition is open even if isComposing is false', () => {
    guard.onCompositionStart();
    expect(guard.isBlocked({ isComposing: false, keyCode: 13 })).toBe(true);
  });

  it('blocks the WebKit confirming Enter after compositionend before the next tick', () => {
    guard.onCompositionStart();
    guard.onCompositionEnd();
    expect(guard.isBlocked({ isComposing: false, keyCode: 13 })).toBe(true);
    vi.runAllTimers();
    expect(guard.isBlocked({ isComposing: false, keyCode: 13 })).toBe(false);
  });

  it('keeps the guard armed if composition restarts before the reset tick', () => {
    guard.onCompositionStart();
    guard.onCompositionEnd();
    guard.onCompositionStart();
    vi.runAllTimers();
    expect(guard.isBlocked({ isComposing: false, keyCode: 13 })).toBe(true);
  });
});
