// @vitest-environment node
import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect } from 'vitest';
import { listFrontendSourceFiles, readFrontendJs } from '../helpers/read-frontend-js.js';

const fixtureRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const globalSearchPath = join(fixtureRoot, 'frontend/src/components/global-search.js');
const mainJsPath = join(fixtureRoot, 'frontend/src/boot.ts');

const frontendSourceFiles = [
  ...listFrontendSourceFiles(join(fixtureRoot, 'frontend/src')),
  ...listFrontendSourceFiles(join(fixtureRoot, 'frontend/src')),
];
const mainJs = readFileSync(mainJsPath, 'utf8');

describe('global-search cleanup (TAC-7)', () => {
  it('deletes frontend/src/components/global-search.js', () => {
    expect(existsSync(globalSearchPath)).toBe(false);
  });

  it('main.js has no global-search import or initGlobalSearch', () => {
    expect(mainJs).not.toMatch(/global-search/);
    expect(mainJs).not.toMatch(/initGlobalSearch/);
  });

  it('main.js has no legacy single-wrap selectors or mode-pill', () => {
    expect(mainJs).not.toMatch(/gs-wrap/);
    expect(mainJs).not.toMatch(/gs-input/);
    expect(mainJs).not.toMatch(/gs-dropdown/);
    expect(mainJs).not.toMatch(/gs-mode-pill/);
    expect(mainJs).not.toMatch(/_detectMode/);
  });

  it('frontend JS / TS has no initGlobalSearch or _detectMode references', () => {
    for (const file of frontendSourceFiles) {
      const src = readFileSync(file, 'utf8');
      expect(src, file).not.toMatch(/initGlobalSearch/);
      expect(src, file).not.toMatch(/_detectMode/);
    }
  });
});

describe('global-search cleanup regression (TAC-5)', () => {
  it('knowledge search remains mounted from viewer (no global-search import)', () => {
    const src = readFrontendJs('frontend/src/knowledge/ui/knowledge-search.tsx');
    expect(src).toMatch(/export function mountKnowledgeSearch/);
    expect(src).not.toMatch(/initGlobalSearch/);
  });
});
