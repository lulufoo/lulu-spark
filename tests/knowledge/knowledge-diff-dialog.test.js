// @vitest-environment jsdom
import { createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { flushSync } from 'react-dom';
import { describe, it, expect, beforeEach, afterEach, vi } from 'vitest';

const apiMocks = vi.hoisted(() => ({
  fetchKbStatus: vi.fn(),
  commitKbFile: vi.fn(),
  revertKbFile: vi.fn(),
}));

const toastMocks = vi.hoisted(() => ({
  showToast: vi.fn(),
}));

vi.mock('../../frontend/src/host/api.ts', () => ({
  fetchKbStatus: (...args) => apiMocks.fetchKbStatus(...args),
  commitKbFile: (...args) => apiMocks.commitKbFile(...args),
  revertKbFile: (...args) => apiMocks.revertKbFile(...args),
}));

vi.mock('../../frontend/src/toast.tsx', () => ({
  showToast: (...args) => toastMocks.showToast(...args),
}));

function deferred() {
  let resolve;
  const promise = new Promise((r) => {
    resolve = r;
  });
  return { promise, resolve };
}

async function loadModule() {
  vi.resetModules();
  const mod = await import('../../frontend/src/knowledge/ui/knowledge-diff-dialog.tsx');
  document.body.innerHTML = '<div id="kb-diff-host"></div>';
  const root = createRoot(document.getElementById('kb-diff-host'));
  flushSync(() => root.render(createElement(mod.KnowledgeDiffDialog)));
  return mod;
}

describe('knowledge diff dialog', () => {
  beforeEach(() => {
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
    document.body.innerHTML = '';
  });

  it('openKnowledgeDiffDialog fetches status and lists New files', async () => {
    const { openKnowledgeDiffDialog } = await loadModule();
    await openKnowledgeDiffDialog('owner/knowledge');
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
    const { openKnowledgeDiffDialog } = await loadModule();
    await openKnowledgeDiffDialog('owner/knowledge');
    expect(document.getElementById('kb-diff-file-list').textContent).toContain('No local changes');
    expect(document.getElementById('btn-kb-diff-ok').disabled).toBe(true);
  });

  it('Commit closes immediately and does not wait for the API', async () => {
    const pending = deferred();
    apiMocks.commitKbFile.mockReturnValue(pending.promise);
    const { openKnowledgeDiffDialog } = await loadModule();
    await openKnowledgeDiffDialog('owner/knowledge');
    document.getElementById('kb-diff-msg').value = 'docs: update';
    document.getElementById('btn-kb-diff-ok').click();
    expect(document.getElementById('kb-diff-dialog').classList.contains('open')).toBe(false);
    expect(apiMocks.commitKbFile).toHaveBeenCalledWith('owner/knowledge', 'docs: update');
    expect(toastMocks.showToast).not.toHaveBeenCalled();
    pending.resolve({});
    await Promise.resolve();
    expect(toastMocks.showToast).toHaveBeenCalledWith('✓ Committed and pushed', 'success');
  });

  it('Discard closes immediately and toasts after revert', async () => {
    const pending = deferred();
    apiMocks.revertKbFile.mockReturnValue(pending.promise);
    const { openKnowledgeDiffDialog } = await loadModule();
    await openKnowledgeDiffDialog('owner/knowledge');
    document.getElementById('btn-kb-diff-revert-all').click();
    expect(document.getElementById('kb-diff-dialog').classList.contains('open')).toBe(false);
    pending.resolve({});
    await Promise.resolve();
    expect(toastMocks.showToast).toHaveBeenCalledWith('✓ Local changes discarded', 'success');
  });

  it('closeKnowledgeDiffDialog removes the open class', async () => {
    const { openKnowledgeDiffDialog, closeKnowledgeDiffDialog } = await loadModule();
    await openKnowledgeDiffDialog('owner/knowledge');
    closeKnowledgeDiffDialog();
    expect(document.getElementById('kb-diff-dialog').classList.contains('open')).toBe(false);
  });
});
