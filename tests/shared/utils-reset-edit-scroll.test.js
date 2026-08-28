// @vitest-environment jsdom
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { resetEditAreaScroll } from '../../frontend/src/shared/utils.ts';

function makeEditArea() {
  const el = document.createElement('textarea');
  el.value = 'line one\nline two\nline three';
  el.scrollTop = 500;
  el.setSelectionRange = vi.fn();
  el.focus = vi.fn(function focusMock() {
    this.scrollTop = 999;
  });
  return el;
}

describe('resetEditAreaScroll', () => {
  beforeEach(() => {
    globalThis.requestAnimationFrame = (fn) => fn();
  });

  it('resets scrollTop and calls setSelectionRange(0, 0)', () => {
    const el = makeEditArea();
    resetEditAreaScroll(el);
    expect(el.scrollTop).toBe(0);
    expect(el.setSelectionRange).toHaveBeenCalledWith(0, 0);
    expect(el.focus).not.toHaveBeenCalled();
  });

  it('with focus: true, resets scroll after focus-induced scroll', () => {
    const el = makeEditArea();
    resetEditAreaScroll(el, { focus: true });
    expect(el.focus).toHaveBeenCalled();
    expect(el.scrollTop).toBe(0);
    expect(el.setSelectionRange).toHaveBeenCalledWith(0, 0);
  });

  it('uses focus({ preventScroll: true }) when supported', () => {
    const el = makeEditArea();
    el.focus = vi.fn(function focusWithPreventScroll(opts) {
      expect(opts).toEqual({ preventScroll: true });
      this.scrollTop = 999;
    });
    resetEditAreaScroll(el, { focus: true });
    expect(el.scrollTop).toBe(0);
  });

  it('falls back to plain focus() when preventScroll throws', () => {
    const el = makeEditArea();
    const plainFocus = vi.fn(function plainFocusMock() {
      this.scrollTop = 888;
    });
    el.focus = vi.fn(function focusThrows(opts) {
      if (opts && opts.preventScroll) throw new TypeError('preventScroll not supported');
      plainFocus.call(this);
    });
    resetEditAreaScroll(el, { focus: true });
    expect(plainFocus).toHaveBeenCalled();
    expect(el.scrollTop).toBe(0);
  });

  it('returns safely when editArea is null or undefined', () => {
    expect(() => resetEditAreaScroll(null)).not.toThrow();
    expect(() => resetEditAreaScroll(undefined)).not.toThrow();
  });

  it('absorbs setSelectionRange errors', () => {
    const el = makeEditArea();
    el.setSelectionRange = vi.fn(() => {
      throw new DOMException('InvalidStateError');
    });
    expect(() => resetEditAreaScroll(el)).not.toThrow();
    expect(el.scrollTop).toBe(0);
  });
});
