import type { SketchAppearance } from '../commands/sketch-preview.ts';

const painters = new Set<() => void>();
let observer: MutationObserver | undefined;

export function currentSketchAppearance(): SketchAppearance {
  const sheet = document.getElementById('theme-sheet');
  if (sheet instanceof HTMLLinkElement) {
    const href = sheet.getAttribute('href') || sheet.href || '';
    if (sheet.dataset.themeId === 'dark' || href.includes('theme-dark')) return 'dark';
    if (sheet.dataset.themeId === 'light' || href.includes('theme-light')) return 'light';
  }
  return document.documentElement.dataset.theme === 'dark' ? 'dark' : 'light';
}

function paintAll() {
  for (const paint of painters) paint();
}

export function watchSketchTheme(paint: () => void): () => void {
  painters.add(paint);
  if (painters.size === 1) {
    document.addEventListener('spark-theme', paintAll);
    if (typeof MutationObserver === 'function') {
      observer = new MutationObserver(paintAll);
      observer.observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme'] });
      const sheet = document.getElementById('theme-sheet');
      if (sheet) observer.observe(sheet, { attributes: true, attributeFilter: ['href', 'data-theme-id'] });
    }
  }
  return () => {
    painters.delete(paint);
    if (painters.size) return;
    document.removeEventListener('spark-theme', paintAll);
    observer?.disconnect();
    observer = undefined;
  };
}
