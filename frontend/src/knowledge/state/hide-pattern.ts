export const KB_HIDE_PATTERN_KEY = 'kb_hide_pattern';

type CompileOk = { ok: true; regex: RegExp };
type CompileErr = { ok: false; error: string };
type CompileResult = CompileOk | CompileErr;

function compilePattern(pattern: string): CompileResult {
  try {
    return { ok: true, regex: new RegExp(pattern) };
  } catch (err) {
    return { ok: false, error: err instanceof Error ? err.message : 'Invalid regular expression' };
  }
}

export function getKbHidePattern(): string {
  try {
    return localStorage.getItem(KB_HIDE_PATTERN_KEY) || '';
  } catch {
    return '';
  }
}

export function validateKbHidePattern(pattern: string | null | undefined): { ok: true } | CompileErr {
  const value = pattern ?? '';
  if (value === '') {
    return { ok: true };
  }
  const compiled = compilePattern(value);
  return compiled.ok ? { ok: true } : compiled;
}

export function saveKbHidePattern(pattern: string | null | undefined): { ok: true } | CompileErr {
  const validation = validateKbHidePattern(pattern);
  if (!validation.ok) {
    return validation;
  }
  try {
    localStorage.setItem(KB_HIDE_PATTERN_KEY, pattern ?? '');
  } catch (err) {
    return { ok: false, error: err instanceof Error ? err.message : 'Failed to save pattern' };
  }
  window.dispatchEvent(new CustomEvent('kb:hide-pattern-changed'));
  return { ok: true };
}

export function shouldHideEntry(name: string, pattern: string): boolean {
  if (!pattern) {
    return false;
  }
  const compiled = compilePattern(pattern);
  return compiled.ok ? compiled.regex.test(name) : false;
}
