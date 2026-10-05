import { existsSync, readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { describe, expect, it } from 'vitest';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');

describe('knowledge git surfaces removed', () => {
  it('deletes the knowledge Diff / Commit dialogs', () => {
    expect(existsSync(join(repoRoot, 'frontend/src/knowledge/ui/knowledge-diff-dialog.tsx'))).toBe(false);
    expect(existsSync(join(repoRoot, 'frontend/src/knowledge/ui/viewer/commit.tsx'))).toBe(false);
    expect(existsSync(join(repoRoot, 'frontend/src/knowledge/commands/viewer/commit.ts'))).toBe(false);
    expect(existsSync(join(repoRoot, 'frontend/src/knowledge/state/commit.ts'))).toBe(false);
  });

  it('does not export knowledge git host APIs', () => {
    const knowledgeApi = readFileSync(join(repoRoot, 'frontend/src/host/api/knowledge.ts'), 'utf8');
    const api = readFileSync(join(repoRoot, 'frontend/src/host/api.ts'), 'utf8');
    for (const name of ['commitKbFile', 'fetchKbStatus', 'fetchKbDiffStatus', 'revertKbFile']) {
      expect(knowledgeApi).not.toMatch(new RegExp(`\\b${name}\\b`));
      expect(api).not.toMatch(new RegExp(`\\b${name}\\b`));
    }
  });
});
