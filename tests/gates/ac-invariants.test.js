import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, it, expect } from 'vitest';
import { readNotesViewerSource } from '../helpers/read-frontend-js.js';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

describe('AC invariants', () => {
  it('I1: create/edit share the same viewer.js shell (no second editor module)', async () => {
    const { readFileSync, existsSync } = await import('node:fs');
    const viewer = readNotesViewerSource();
    expect(viewer).toMatch(/export async function openDoc\s*\(/);
    expect(viewer).toMatch(/export async function openCreateNote\s*\(/);
    expect(existsSync(join(repoRoot, 'frontend/js/components/note-editor.js'))).toBe(false);
    expect(existsSync(join(repoRoot, 'frontend/js/note-editor.js'))).toBe(false);
  });

  it('router tests include corpus ?path= deep link coverage', async () => {
    const { readFileSync } = await import('node:fs');
    const routerTest = readFileSync(join(repoRoot, 'tests/router/index.test.js'), 'utf8');
    expect(routerTest).toMatch(/parseHash\('#\/corpus\/owner\/repo\?path=docs\/guide\.md'\)/);
  });

  it('corpus-doc-list tests cover in-tree open + mountKbReader mock', async () => {
    const { readFileSync } = await import('node:fs');
    const corpusTest = readFileSync(join(repoRoot, 'tests/corpus/corpus-doc-list.test.js'), 'utf8');
    expect(corpusTest).toMatch(/mountKbReader/);
    expect(corpusTest).toMatch(/replaceState|syncCorpusHash/);
  });
});
