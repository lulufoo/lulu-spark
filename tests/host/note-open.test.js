import { readFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { beforeEach, describe, expect, it, vi } from 'vitest';

vi.mock('../../frontend/src/host/apiClient.ts', async (importOriginal) => ({
  ...(await importOriginal()),
  invoke: vi.fn(),
}));

import { invoke as invokeCommand } from '../../frontend/src/host/apiClient.ts';
import { resolveNoteForOpen } from '../../frontend/src/host/api.ts';

const repoRoot = join(dirname(fileURLToPath(import.meta.url)), '../..');
const ID = 'abcdefabcdefabcdefabcdefabcdefab';

beforeEach(() => {
  vi.mocked(invokeCommand).mockReset();
});

describe('resolveNoteForOpen host api', () => {
  it('invokes resolve_note_for_open with the id and returns the entry', async () => {
    const entry = { id: ID, ok: true, common_path: 'ai/note.md', created_at: '202606190004' };
    vi.mocked(invokeCommand).mockResolvedValue(entry);
    await expect(resolveNoteForOpen(ID)).resolves.toEqual(entry);
    expect(invokeCommand).toHaveBeenCalledWith('resolve_note_for_open', { id: ID });
  });

  it('rejects when Rust reports an unknown or malformed id', async () => {
    vi.mocked(invokeCommand).mockResolvedValue({ id: ID, ok: false, error: 'Entry not found' });
    await expect(resolveNoteForOpen(ID)).rejects.toThrow('Entry not found');
  });

  it('goes through the shared invoke wrapper, not a raw tauri import', () => {
    const src = readFileSync(join(repoRoot, 'frontend/src/host/api/note-open.ts'), 'utf8');
    expect(src).toMatch(/invoke\('resolve_note_for_open'/);
    expect(src).not.toMatch(/@tauri-apps\//);
  });
});
