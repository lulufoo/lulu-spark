// @vitest-environment jsdom
import { describe, it, expect, beforeEach } from 'vitest';
import {
  getKbHidePatternStrings,
  setKbHidePatternsCache,
  validateKbHidePattern,
  shouldHideEntry,
} from '../../frontend/src/knowledge/state/hide-pattern.ts';

describe('knowledge-hide-pattern', () => {
  beforeEach(() => {
    setKbHidePatternsCache([]);
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
    it('returns true when name matches any pattern', () => {
      expect(shouldHideEntry('foo.xxx', ['\\.xxx$', '^\\.git$'])).toBe(true);
    });

    it('returns false when name does not match pattern', () => {
      expect(shouldHideEntry('foo.md', ['\\.xxx$'])).toBe(false);
    });

    it('returns false for empty pattern list', () => {
      expect(shouldHideEntry('foo.xxx', [])).toBe(false);
    });

    it('returns false when RegExp parse fails (I7 degrade)', () => {
      expect(shouldHideEntry('foo.xxx', ['['])).toBe(false);
    });
  });

  describe('getKbHidePatternStrings', () => {
    it('returns empty list when unset', () => {
      expect(getKbHidePatternStrings()).toEqual([]);
    });

    it('reads cached rows', () => {
      setKbHidePatternsCache([{ id: 'a', pattern: '\\.xxx$' }]);
      expect(getKbHidePatternStrings()).toEqual(['\\.xxx$']);
    });
  });
});
