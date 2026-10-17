/**
 * Resolve the translation lang for a document file read (language + layer aware).
 * Only raw layer uses translations.zh when lang is zh.
 */
export function docFileLang(
  entry: { translations?: { zh?: string } },
  lang: string,
  layer = 'raw',
): string | undefined {
  if (lang === 'zh' && entry.translations?.zh && layer === 'raw') {
    return 'zh';
  }
  return undefined;
}
