import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../frontend/src/host/apiClient.ts', async (importOriginal) => ({
  ...(await importOriginal()),
  invoke: vi.fn(),
}));

import { invoke as invokeCommand } from '../../frontend/src/host/apiClient.ts';
import { resolveKnowledgeForOpen } from '../../frontend/src/host/api.ts';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const ID = 'abcdefabcdefabcdefabcdefabcdefab';

beforeEach(() => {
  vi.mocked(invokeCommand).mockReset();
});

describe('resolveKnowledgeForOpen host api', () => {
  it('invokes resolve_knowledge_for_open with the id and returns repo and path', async () => {
    const doc = { id: ID, ok: true, repo: 'demo', path: 'guide/intro.md' };
    vi.mocked(invokeCommand).mockResolvedValue(doc);
    await expect(resolveKnowledgeForOpen(ID)).resolves.toEqual(doc);
    expect(invokeCommand).toHaveBeenCalledWith('resolve_knowledge_for_open', { id: ID });
  });

  it('rejects when Rust reports an unknown id', async () => {
    vi.mocked(invokeCommand).mockResolvedValue({ id: ID, ok: false, error: 'Unknown document id' });
    await expect(resolveKnowledgeForOpen(ID)).rejects.toThrow('Unknown document id');
  });

  it('goes through the shared invoke wrapper, not a raw tauri import', () => {
    const src = readFileSync(join(repoRoot, 'frontend/src/host/api/knowledge-open.ts'), 'utf8');
    expect(src).toMatch(/invoke\('resolve_knowledge_for_open'/);
    expect(src).not.toMatch(/@tauri-apps\//);
  });
});
