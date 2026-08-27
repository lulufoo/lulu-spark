// @vitest-environment jsdom
import { describe, it, expect, beforeEach, vi } from 'vitest';
import {
  KB_HIDE_PATTERN_KEY,
  getKbHidePattern,
  validateKbHidePattern,
  saveKbHidePattern,
  shouldHideEntry,
} from '../../frontend/js/corpus/corpus-hide-pattern.js';

function installLocalStorageMock() {
  const store = {};
  globalThis.localStorage = {
    getItem(key) {
      return Object.prototype.hasOwnProperty.call(store, key) ? store[key] : null;
    },
    setItem(key, value) {
      store[key] = String(value);
    },
    removeItem(key) {
      delete store[key];
    },
    clear() {
      for (const key of Object.keys(store)) delete store[key];
    },
  };
}

describe('corpus-hide-pattern', () => {
  beforeEach(() => {
    installLocalStorageMock();
  });

  it('exports KB_HIDE_PATTERN_KEY as kb_hide_pattern', () => {
    expect(KB_HIDE_PATTERN_KEY).toBe('kb_hide_pattern');
  });

  describe('validateKbHidePattern', () => {
    it('accepts valid pattern', () => {
      expect(validateKbHidePattern('\\.xxx$')).toEqual({ ok: true });
    });

    it('accepts empty pattern', () => {
      expect(validateKbHidePattern('')).toEqual({ ok: true });
    });

    it('rejects invalid regex syntax', () => {
      const result = validateKbHidePattern('[');
      expect(result.ok).toBe(false);
      expect(result.error).toBeTruthy();
    });
  });

  describe('shouldHideEntry', () => {
    it('returns true when name matches pattern', () => {
      expect(shouldHideEntry('foo.xxx', '\\.xxx$')).toBe(true);
    });

    it('returns false when name does not match pattern', () => {
      expect(shouldHideEntry('foo.md', '\\.xxx$')).toBe(false);
    });

    it('returns false for empty pattern', () => {
      expect(shouldHideEntry('foo.xxx', '')).toBe(false);
    });

    it('returns false when RegExp parse fails (I7 degrade)', () => {
      expect(shouldHideEntry('foo.xxx', '[')).toBe(false);
    });
  });

  describe('getKbHidePattern', () => {
    it('returns empty string when unset', () => {
      expect(getKbHidePattern()).toBe('');
    });

    it('reads stored pattern from localStorage', () => {
      localStorage.setItem(KB_HIDE_PATTERN_KEY, '\\.xxx$');
      expect(getKbHidePattern()).toBe('\\.xxx$');
    });
  });

  describe('saveKbHidePattern', () => {
    it('writes valid pattern to localStorage and dispatches event', () => {
      const handler = vi.fn();
      window.addEventListener('kb:hide-pattern-changed', handler);

      const result = saveKbHidePattern('\\.xxx$');
      expect(result).toEqual({ ok: true });
      expect(localStorage.getItem(KB_HIDE_PATTERN_KEY)).toBe('\\.xxx$');
      expect(handler).toHaveBeenCalledTimes(1);

      window.removeEventListener('kb:hide-pattern-changed', handler);
    });

    it('rejects invalid pattern without writing localStorage', () => {
      localStorage.setItem(KB_HIDE_PATTERN_KEY, '\\.old$');
      const handler = vi.fn();
      window.addEventListener('kb:hide-pattern-changed', handler);

      const result = saveKbHidePattern('[');
      expect(result.ok).toBe(false);
      expect(result.error).toBeTruthy();
      expect(localStorage.getItem(KB_HIDE_PATTERN_KEY)).toBe('\\.old$');
      expect(handler).not.toHaveBeenCalled();

      window.removeEventListener('kb:hide-pattern-changed', handler);
    });

    it('clears pattern when saving empty string', () => {
      localStorage.setItem(KB_HIDE_PATTERN_KEY, '\\.xxx$');
      const handler = vi.fn();
      window.addEventListener('kb:hide-pattern-changed', handler);

      const result = saveKbHidePattern('');
      expect(result).toEqual({ ok: true });
      expect(localStorage.getItem(KB_HIDE_PATTERN_KEY)).toBe('');
      expect(handler).toHaveBeenCalledTimes(1);

      window.removeEventListener('kb:hide-pattern-changed', handler);
    });
  });
});
