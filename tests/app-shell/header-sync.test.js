// @vitest-environment jsdom
import { describe, it, expect, beforeEach, vi } from 'vitest';

vi.mock('../../frontend/js/host/api.js', () => ({
  reindexKbRepo: vi.fn(),
  getReindexStatus: vi.fn(),
}));

vi.mock('../../frontend/js/corpus/corpus-diff-dialog.js', () => ({
  openKbDiffDialog: vi.fn(),
}));

vi.mock('../../frontend/js/app-shell/commit-dialog.js', () => ({
  openCommitChangesDialog: vi.fn(),
}));

import * as api from '../../frontend/js/host/api.js';
import { openKbDiffDialog } from '../../frontend/js/corpus/corpus-diff-dialog.js';
import { openCommitChangesDialog } from '../../frontend/js/app-shell/commit-dialog.js';
import {
  initHeaderSync,
  setHeaderSyncCorpusContext,
  clearHeaderSyncCorpusContext,
} from '../../frontend/js/app-shell/header-sync.js';

describe('header-sync', () => {
  beforeEach(() => {
    clearHeaderSyncCorpusContext();
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

  it('routes commit to kb diff dialog when corpus repo is active', () => {
    setHeaderSyncCorpusContext('owner/repo', vi.fn());
    document.getElementById('btn-push-index').click();
    expect(openKbDiffDialog).toHaveBeenCalledWith('owner/repo');
    expect(openCommitChangesDialog).not.toHaveBeenCalled();
  });

  it('routes commit to workbench dialog when no corpus repo is active', () => {
    document.getElementById('btn-push-index').click();
    expect(openCommitChangesDialog).toHaveBeenCalledTimes(1);
    expect(openKbDiffDialog).not.toHaveBeenCalled();
  });

  it('routes local refresh to corpus callback when corpus repo is active', async () => {
    const onRefresh = vi.fn().mockResolvedValue(undefined);
    setHeaderSyncCorpusContext('owner/repo', onRefresh);
    document.getElementById('btn-local-refresh').click();
    await Promise.resolve();
    expect(onRefresh).toHaveBeenCalledTimes(1);
  });

  it('routes pull to kb reindex when corpus repo is active', async () => {
    api.reindexKbRepo.mockResolvedValue(undefined);
    api.getReindexStatus.mockResolvedValue({ status: 'idle' });
    setHeaderSyncCorpusContext('owner/repo', vi.fn());
    document.getElementById('btn-pull').click();
    await Promise.resolve();
    expect(api.reindexKbRepo).toHaveBeenCalledWith('owner/repo');
  });
});
