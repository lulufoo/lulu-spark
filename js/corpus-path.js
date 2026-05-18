/**
 * Resolve the active file path for an index entry (language + layer aware).
 * Only raw layer uses translations.zh when lang is zh.
 */
export function getActivePath(entry, lang, layer = 'raw') {
  if (lang === 'zh' && entry.translations?.zh && layer === 'raw') {
    return entry.translations.zh;
  }
  return entry.common_path;
}
