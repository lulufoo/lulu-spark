// @vitest-environment jsdom
import { describe, it, expect, beforeEach, vi } from 'vitest';

vi.mock('../../frontend/src/host/api.ts', () => ({
  reindexKbRepo: vi.fn(),
  getReindexStatus: vi.fn(),
}));

vi.mock('../../frontend/src/knowledge/ui/diff-dialog.tsx', () => ({
  openKbDiffDialog: vi.fn(),
}));

vi.mock('../../frontend/src/app-shell/commands/commit-dialog.ts', () => ({
  openCommitChangesDialog: vi.fn(),
}));

import * as api from '../../frontend/src/host/api.ts';
import { openKbDiffDialog } from '../../frontend/src/knowledge/ui/diff-dialog.tsx';
import { openCommitChangesDialog } from '../../frontend/src/app-shell/commands/commit-dialog.ts';
import {
  initHeaderSync,
  setHeaderSyncKnowledgeContext,
  clearHeaderSyncKnowledgeContext,
} from '../../frontend/src/app-shell/commands/header-sync.ts';

describe('header-sync', () => {
  beforeEach(() => {
    clearHeaderSyncKnowledgeContext();
    vi.clearAllMocks();
    document.body.innerHTML = `
      <button id="btn-push-index">↑ Commit changes</button>
      <button id="btn-pull">↓ Update project</button>
      <button id="btn-local-refresh">⟳ Refresh local</button>
    `;
    initHeaderSync({
      pullProject: vi.fn(),
      loadIndex: vi.fn(),
      openWorkbenchCommit: openCommitChangesDialog,
    });
  });

  it('routes commit to kb diff dialog when knowledge repo is active', () => {
    setHeaderSyncKnowledgeContext('owner/repo', vi.fn());
    document.getElementById('btn-push-index').click();
    expect(openKbDiffDialog).toHaveBeenCalledWith('owner/repo');
    expect(openCommitChangesDialog).not.toHaveBeenCalled();
  });

  it('routes commit to workbench dialog when no knowledge repo is active', () => {
    document.getElementById('btn-push-index').click();
    expect(openCommitChangesDialog).toHaveBeenCalledTimes(1);
    expect(openKbDiffDialog).not.toHaveBeenCalled();
  });

  it('routes local refresh to knowledge callback when knowledge repo is active', async () => {
    const onRefresh = vi.fn().mockResolvedValue(undefined);
    setHeaderSyncKnowledgeContext('owner/repo', onRefresh);
    document.getElementById('btn-local-refresh').click();
    await Promise.resolve();
    expect(onRefresh).toHaveBeenCalledTimes(1);
  });

  it('routes pull to kb reindex when knowledge repo is active', async () => {
    api.reindexKbRepo.mockResolvedValue(undefined);
    api.getReindexStatus.mockResolvedValue({ status: 'idle' });
    setHeaderSyncKnowledgeContext('owner/repo', vi.fn());
    document.getElementById('btn-pull').click();
    await Promise.resolve();
    expect(api.reindexKbRepo).toHaveBeenCalledWith('owner/repo');
  });
});
