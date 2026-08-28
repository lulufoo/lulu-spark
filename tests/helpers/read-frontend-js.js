import { existsSync, readdirSync, readFileSync, statSync } from 'node:fs';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

export const FRONTEND_SOURCE_EXT = new Set(['.js', '.ts', '.tsx']);

/** Absolute paths of `.js` / `.ts` / `.tsx` under `dir`, recursive. */
export function listFrontendSourceFiles(dir) {
  const out = [];
  if (!existsSync(dir)) return out;
  for (const name of readdirSync(dir)) {
    const abs = join(dir, name);
    if (statSync(abs).isDirectory()) {
      out.push(...listFrontendSourceFiles(abs));
    } else if (FRONTEND_SOURCE_EXT.has(name.slice(name.lastIndexOf('.')))) {
      out.push(abs);
    }
  }
  return out;
}

/** Concatenate every source file in a directory (sorted, one level). */
export function readJsDir(dir) {
  return readdirSync(dir)
    .filter((name) => FRONTEND_SOURCE_EXT.has(name.slice(name.lastIndexOf('.'))))
    .sort()
    .map((name) => {
      const abs = join(dir, name);
      const text = readFileSync(abs, 'utf8');
      return [text, ...followSrcReexports(abs, text)].join('\n');
    })
    .join('\n');
}

/**
 * Read a frontend JS file. If a same-name folder exists
 * (`foo.js` + `foo/`), concatenate the folder too.
 */
function followSrcReexports(fromAbs, text, seen = new Set()) {
  const extras = [];
  const fromDir = dirname(fromAbs);
  const stub = text.trim().startsWith('export') && text.split('\n').length <= 16;
  for (const match of text.matchAll(/from\s+['"]([^'"]+)['"]/g)) {
    const spec = match[1];
    // Follow stub re-exports only. Implementation `.ts`/`.tsx` imports would
    // otherwise pull the whole graph into source-scan tests.
    if (!stub) continue;
    const resolved = resolve(fromDir, spec);
    if (!existsSync(resolved) || seen.has(resolved)) continue;
    seen.add(resolved);
    const next = readFileSync(resolved, 'utf8');
    extras.push(next);
    extras.push(...followSrcReexports(resolved, next, seen));
  }
  return extras;
}

/**
 * Read a frontend JS file. If a same-name folder exists
 * (`foo.js` + `foo/`), concatenate the folder too.
 * Re-exports into `frontend/src` are followed so source-scan tests
 * still see the React/TS implementation.
 */
export function readFrontendJs(rel) {
  const abs = join(repoRoot, rel);
  const parts = [];
  let fileAbs = abs;
  if (!existsSync(fileAbs) && rel.endsWith('.js')) {
    const tsxAbs = abs.replace(/\.js$/, '.tsx');
    const tsAbs = abs.replace(/\.js$/, '.ts');
    if (existsSync(tsxAbs)) fileAbs = tsxAbs;
    else if (existsSync(tsAbs)) fileAbs = tsAbs;
  }
  if (existsSync(fileAbs) && statSync(fileAbs).isFile()) {
    const text = readFileSync(fileAbs, 'utf8');
    parts.push(text);
    parts.push(...followSrcReexports(fileAbs, text));
  }
  const companion = abs.replace(/\.(js|tsx|ts)$/, '');
  if (existsSync(companion) && statSync(companion).isDirectory()) {
    parts.push(readJsDir(companion));
  }
  return parts.join('\n');
}

/** Concatenate every source file under `frontend/src` (React / TS entry). */
export function readFrontendSrcTree() {
  return listFrontendSourceFiles(join(repoRoot, 'frontend/src'))
    .sort()
    .map((abs) => readFileSync(abs, 'utf8'))
    .join('\n');
}

/** App bootstrap: boot.js plus the modules extracted from the old main.js. */
export function readMainSource() {
  return [
    readFrontendJs('frontend/src/boot.ts'),
    readFrontendJs('frontend/src/app-shell/sediment-kb.tsx'),
    readFrontendJs('frontend/src/app-shell/routes.ts'),
    readFrontendJs('frontend/src/app-shell/skills-dialog.tsx'),
    readFrontendJs('frontend/src/app-shell/tooltip.ts'),
  ].join('\n');
}

export function readSettingsDialogSource() {
  const settingsDir = join(repoRoot, 'frontend/src/app-shell/settings');
  return [
    readFrontendJs('frontend/src/app-shell/settings-dialog.tsx'),
    ...listFrontendSourceFiles(settingsDir)
      .sort()
      .map((abs) => readFileSync(abs, 'utf8')),
  ].join('\n');
}

export function readHostApiSource() {
  return readFrontendJs('frontend/src/host/api.ts');
}

export function readNotesViewerSource() {
  return readFrontendJs('frontend/src/notes/viewer.ts');
}

export function readCorpusViewerSource() {
  return readFrontendJs('frontend/src/corpus/corpus-viewer.ts');
}

/** Desktop chrome: React shell JSX, dialogs it mounts, plus remaining index.html IDs. */
export function readShellHtml() {
  const shellAbs = join(repoRoot, 'frontend/src/shell.tsx');
  const shell = readFileSync(shellAbs, 'utf8');
  const extras = [];
  const seen = new Set([shellAbs]);
  for (const match of shell.matchAll(/from\s+['"](\.[^'"]+)['"]/g)) {
    const resolved = resolve(dirname(shellAbs), match[1]);
    if (!existsSync(resolved) || seen.has(resolved)) continue;
    seen.add(resolved);
    extras.push(readFileSync(resolved, 'utf8'));
  }
  return [
    shell,
    ...extras,
    readFileSync(join(repoRoot, 'frontend/index.html'), 'utf8'),
  ].join('\n');
}

export { repoRoot };
