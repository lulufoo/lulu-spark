export const KB_HIDE_PATTERN_KEY = 'kb_hide_pattern';

/**
 * @param {string} pattern
 * @returns {{ ok: true, regex: RegExp } | { ok: false, error: string }}
 */
function compilePattern(pattern) {
  try {
    return { ok: true, regex: new RegExp(pattern) };
  } catch (err) {
    return { ok: false, error: err?.message || 'Invalid regular expression' };
  }
}

/**
 * @returns {string}
 */
export function getKbHidePattern() {
  try {
    return localStorage.getItem(KB_HIDE_PATTERN_KEY) || '';
  } catch {
    return '';
  }
}

/**
 * @param {string} pattern
 * @returns {{ ok: true } | { ok: false, error: string }}
 */
export function validateKbHidePattern(pattern) {
  const value = pattern ?? '';
  if (value === '') {
    return { ok: true };
  }
  const compiled = compilePattern(value);
  return compiled.ok ? { ok: true } : compiled;
}

/**
 * @param {string} pattern
 * @returns {{ ok: true } | { ok: false, error: string }}
 */
export function saveKbHidePattern(pattern) {
  const validation = validateKbHidePattern(pattern);
  if (!validation.ok) {
    return validation;
  }
  try {
    localStorage.setItem(KB_HIDE_PATTERN_KEY, pattern ?? '');
  } catch (err) {
    return { ok: false, error: err?.message || 'Failed to save pattern' };
  }
  window.dispatchEvent(new CustomEvent('kb:hide-pattern-changed'));
  return { ok: true };
}

/**
 * @param {string} name
 * @param {string} pattern
 * @returns {boolean}
 */
export function shouldHideEntry(name, pattern) {
  if (!pattern) {
    return false;
  }
  const compiled = compilePattern(pattern);
  return compiled.ok ? compiled.regex.test(name) : false;
}
