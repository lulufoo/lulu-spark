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
    readFrontendJs('frontend/src/App.tsx'),
    readFrontendJs('frontend/src/hash-router.tsx'),
    readFrontendJs('frontend/src/shell-pages.tsx'),
    readFrontendJs('frontend/src/app-shell/ui/settings/sediment-kb.tsx'),
    readFrontendJs('frontend/src/app-shell/routes.ts'),
    readFrontendJs('frontend/src/app-shell/ui/skills-dialog.tsx'),
    readFrontendJs('frontend/src/app-shell/ui/tooltip.ts'),
  ].join('\n');
}

export function readSettingsDialogSource() {
  const settingsDirs = [
    join(repoRoot, 'frontend/src/app-shell/ui/settings'),
    join(repoRoot, 'frontend/src/app-shell/commands/settings'),
    join(repoRoot, 'frontend/src/app-shell/state/settings'),
  ];
  return [
    readFrontendJs('frontend/src/app-shell/ui/settings/dialog.tsx'),
    readFrontendJs('frontend/src/app-shell/commands/settings/dialog.ts'),
    ...settingsDirs.flatMap((dir) =>
      listFrontendSourceFiles(dir)
        .sort()
        .map((abs) => readFileSync(abs, 'utf8')),
    ),
  ].join('\n');
}

export function readHostApiSource() {
  return readFrontendJs('frontend/src/host/api.ts');
}

export function readNotesViewerSource() {
  const viewerDirs = [
    join(repoRoot, 'frontend/src/notes/ui/viewer'),
    join(repoRoot, 'frontend/src/notes/commands/viewer'),
  ];
  return [
    readFrontendJs('frontend/src/notes/viewer.ts'),
    ...viewerDirs.flatMap((dir) =>
      listFrontendSourceFiles(dir)
        .sort()
        .map((abs) => readFileSync(abs, 'utf8')),
    ),
  ].join('\n');
}

export function readKnowledgeViewerSource() {
  const viewerDirs = [
    join(repoRoot, 'frontend/src/knowledge/ui/viewer'),
    join(repoRoot, 'frontend/src/knowledge/commands/viewer'),
  ];
  return [
    readFrontendJs('frontend/src/knowledge/viewer.ts'),
    ...viewerDirs.flatMap((dir) =>
      listFrontendSourceFiles(dir)
        .sort()
        .map((abs) => readFileSync(abs, 'utf8')),
    ),
  ].join('\n');
}

/**
 * Desktop chrome: React shell + hash-router page slots + files they import.
 * One hop only — do not walk the whole src graph into source-scan tests.
 */
export function readShellHtml() {
  const entries = [
    join(repoRoot, 'frontend/src/shell.tsx'),
    join(repoRoot, 'frontend/src/hash-router.tsx'),
  ];
  const seen = new Set();
  const parts = [];
  for (const entry of entries) {
    if (!existsSync(entry) || seen.has(entry)) continue;
    seen.add(entry);
    const text = readFileSync(entry, 'utf8');
    parts.push(text);
    for (const match of text.matchAll(/from\s+['"](\.[^'"]+)['"]/g)) {
      const resolved = resolve(dirname(entry), match[1]);
      if (!existsSync(resolved) || seen.has(resolved)) continue;
      seen.add(resolved);
      parts.push(readFileSync(resolved, 'utf8'));
    }
  }
  return [
    ...parts,
    readFileSync(join(repoRoot, 'frontend/index.html'), 'utf8'),
  ].join('\n');
}

export { repoRoot };
