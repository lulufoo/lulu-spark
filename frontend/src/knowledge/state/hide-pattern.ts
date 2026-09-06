export type KbHidePatternRow = { id: string; pattern: string };

type CompileOk = { ok: true; regex: RegExp };
type CompileErr = { ok: false; error: string };
type CompileResult = CompileOk | CompileErr;

let cached: KbHidePatternRow[] = [];

function compilePattern(pattern: string): CompileResult {
  try {
    return { ok: true, regex: new RegExp(pattern) };
  } catch (err) {
    return { ok: false, error: err instanceof Error ? err.message : 'Invalid regular expression' };
  }
}

export function getKbHidePatterns(): KbHidePatternRow[] {
  return cached.slice();
}

export function getKbHidePatternStrings(): string[] {
  return cached.map((row) => row.pattern);
}

export function setKbHidePatternsCache(rows: KbHidePatternRow[]) {
  cached = rows.map((row) => ({ id: row.id, pattern: row.pattern }));
}

export function validateKbHidePattern(pattern: string | null | undefined): { ok: true } | CompileErr {
  const value = (pattern ?? '').trim();
  if (value === '') {
    return { ok: true };
  }
  const compiled = compilePattern(value);
  return compiled.ok ? { ok: true } : compiled;
}

export function shouldHideEntry(name: string, patterns?: string[]): boolean {
  const list = patterns ?? getKbHidePatternStrings();
  for (const pattern of list) {
    if (!pattern) continue;
    const compiled = compilePattern(pattern);
    if (compiled.ok && compiled.regex.test(name)) return true;
  }
  return false;
}

export function emitHidePatternChanged() {
  window.dispatchEvent(new CustomEvent('kb:hide-pattern-changed'));
}
