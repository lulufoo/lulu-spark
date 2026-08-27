// @vitest-environment node
import { execSync } from 'node:child_process';
import { existsSync, readFileSync, readdirSync, statSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect } from 'vitest';

const fixtureRoot = join(dirname(fileURLToPath(import.meta.url)), '..');
const globalSearchPath = join(fixtureRoot, 'frontend/js/components/global-search.js');
const mainJsPath = join(fixtureRoot, 'frontend/js/main.js');
const knowledgeSearchPath = join(fixtureRoot, 'frontend/js/corpus/knowledge-search.js');

function collectJsFiles(dir) {
  const files = [];
  for (const entry of readdirSync(dir)) {
    const full = join(dir, entry);
    if (statSync(full).isDirectory()) {
      files.push(...collectJsFiles(full));
    } else if (entry.endsWith('.js')) {
      files.push(full);
    }
  }
  return files;
}

const frontendJsFiles = collectJsFiles(join(fixtureRoot, 'frontend/js'));
const mainJs = readFileSync(mainJsPath, 'utf8');

describe('global-search cleanup (TAC-7)', () => {
  it('deletes frontend/js/components/global-search.js', () => {
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

  it('frontend JS has no initGlobalSearch or _detectMode references', () => {
    for (const file of frontendJsFiles) {
      const src = readFileSync(file, 'utf8');
      expect(src, file).not.toMatch(/initGlobalSearch/);
      expect(src, file).not.toMatch(/_detectMode/);
    }
  });
});

describe('global-search cleanup regression (TAC-5)', () => {
  it('knowledge-search.js remains mounted from viewer (no global-search import)', () => {
    const src = readFileSync(knowledgeSearchPath, 'utf8');
    expect(src).toMatch(/export function mountKnowledgeSearch/);
    expect(src).not.toMatch(/initGlobalSearch/);
  });
});
