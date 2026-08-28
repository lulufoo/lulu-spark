// @vitest-environment jsdom
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

const apiMocks = vi.hoisted(() => ({
  fetchKbStatus: vi.fn(),
  commitKbFile: vi.fn(),
  revertKbFile: vi.fn(),
}));

vi.mock('../../frontend/src/host/api.ts', () => ({
  fetchKbStatus: (...args) => apiMocks.fetchKbStatus(...args),
  commitKbFile: (...args) => apiMocks.commitKbFile(...args),
  revertKbFile: (...args) => apiMocks.revertKbFile(...args),
}));

function seedDiffDom() {
  document.body.innerHTML = `
    <div id="kb-diff-dialog">
      <div id="kb-diff-dialog-box">
        <div id="kb-diff-dialog-title"></div>
        <div id="kb-diff-file-list"></div>
        <textarea id="kb-diff-msg"></textarea>
        <div id="kb-diff-result"></div>
        <button id="btn-kb-diff-ok">Commit</button>
        <button id="btn-kb-diff-cancel">Cancel</button>
        <button id="btn-kb-diff-revert-all">Revert</button>
      </div>
    </div>
  `;
}

async function loadModule() {
  vi.resetModules();
  seedDiffDom();
  return import('../../frontend/src/corpus/corpus-diff-dialog.tsx');
}

describe('corpus diff dialog', () => {
  beforeEach(() => {
    vi.useFakeTimers();
    vi.clearAllMocks();
    apiMocks.fetchKbStatus.mockResolvedValue({
      new: ['a.md'],
      modified: [],
      renamed: [],
      deleted: [],
      conflicted: [],
      total: 1,
      ahead: 0,
    });
    apiMocks.commitKbFile.mockResolvedValue({});
    apiMocks.revertKbFile.mockResolvedValue({});
  });

  afterEach(() => {
    vi.useRealTimers();
  });

  it('openKbDiffDialog fetches status and lists New files', async () => {
    const { openKbDiffDialog } = await loadModule();
    await openKbDiffDialog('owner/knowledge');
    expect(apiMocks.fetchKbStatus).toHaveBeenCalledWith('owner/knowledge');
    expect(document.getElementById('kb-diff-dialog').classList.contains('open')).toBe(true);
    expect(document.getElementById('kb-diff-dialog-title').textContent).toContain('knowledge');
    expect(document.getElementById('kb-diff-file-list').textContent).toContain('a.md');
    expect(document.getElementById('kb-diff-file-list').textContent).toContain('New');
  });

  it('shows No local changes when status is empty', async () => {
    apiMocks.fetchKbStatus.mockResolvedValue({
      new: [],
      modified: [],
      renamed: [],
      deleted: [],
      conflicted: [],
      total: 0,
      ahead: 0,
    });
    const { openKbDiffDialog } = await loadModule();
    await openKbDiffDialog('owner/knowledge');
    expect(document.getElementById('kb-diff-file-list').textContent).toContain('No local changes');
    expect(document.getElementById('btn-kb-diff-ok').disabled).toBe(true);
  });

  it('Commit calls commitKbFile then closes after success', async () => {
    const { openKbDiffDialog } = await loadModule();
    await openKbDiffDialog('owner/knowledge');
    document.getElementById('kb-diff-msg').value = 'docs: update';
    document.getElementById('btn-kb-diff-ok').click();
    await Promise.resolve();
    expect(apiMocks.commitKbFile).toHaveBeenCalledWith('owner/knowledge', 'docs: update');
    expect(document.getElementById('kb-diff-result').textContent).toContain('Committed and pushed');
    await vi.advanceTimersByTimeAsync(1500);
    expect(document.getElementById('kb-diff-dialog').classList.contains('open')).toBe(false);
  });

  it('closeKbDiffDialog removes the open class', async () => {
    const { openKbDiffDialog, closeKbDiffDialog } = await loadModule();
    await openKbDiffDialog('owner/knowledge');
    closeKbDiffDialog();
    expect(document.getElementById('kb-diff-dialog').classList.contains('open')).toBe(false);
  });
});
