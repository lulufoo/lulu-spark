import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

/** Concatenate every `.js` file in a directory (sorted, one level). */
export function readJsDir(dir) {
  return readdirSync(dir)
    .filter((name) => name.endsWith('.js'))
    .sort()
    .map((name) => readFileSync(join(dir, name), 'utf8'))
    .join('\n');
}

/**
 * Read a frontend JS file. If a same-name folder exists
 * (`foo.js` + `foo/`), concatenate the folder too.
 */
export function readFrontendJs(rel) {
  const abs = join(repoRoot, rel);
  const parts = [];
  if (existsSync(abs) && statSync(abs).isFile()) {
    parts.push(readFileSync(abs, 'utf8'));
  }
  const companion = abs.replace(/\.js$/, '');
  if (existsSync(companion) && statSync(companion).isDirectory()) {
    parts.push(readJsDir(companion));
  }
  return parts.join('\n');
}

/** main.js plus the app-shell modules extracted from it. */
export function readMainSource() {
  return [
    readFrontendJs('frontend/js/main.js'),
    readFrontendJs('frontend/js/app-shell/sediment-kb.js'),
    readFrontendJs('frontend/js/app-shell/routes.js'),
    readFrontendJs('frontend/js/app-shell/skills-dialog.js'),
    readFrontendJs('frontend/js/app-shell/tooltip.js'),
  ].join('\n');
}

export function readSettingsDialogSource() {
  return [
    readFrontendJs('frontend/js/app-shell/settings-dialog.js'),
    readJsDir(join(repoRoot, 'frontend/js/app-shell/settings')),
  ].join('\n');
}

export function readHostApiSource() {
  return readFrontendJs('frontend/js/host/api.js');
}

export function readNotesViewerSource() {
  return readFrontendJs('frontend/js/notes/viewer.js');
}

export function readCorpusViewerSource() {
  return readFrontendJs('frontend/js/corpus/corpus-viewer.js');
}

export { repoRoot };
