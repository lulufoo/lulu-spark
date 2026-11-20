// @vitest-environment jsdom
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { state } from '../../frontend/src/host/state.ts';
import * as api from '../../frontend/src/host/api.ts';
import {
  dropFolderOn,
  dropFolderOnRoot,
  endFolderMove,
  folderAcceptsFolderDrop,
  overFolderTarget,
  startFolderMove,
} from '../../frontend/src/notes/commands/folder-move.ts';
import { emptyFolderMove, folderMoveStore } from '../../frontend/src/notes/state/folder-move.ts';
import { refreshNotesIndex } from '../../frontend/src/notes/commands/reload-index.ts';

vi.mock('../../frontend/src/host/api.ts', () => ({
  moveFolder: vi.fn(),
}));

vi.mock('../../frontend/src/notes/commands/reload-index.ts', () => ({
  refreshNotesIndex: vi.fn(),
}));

function dragEvent(overrides = {}) {
  return {
    dataTransfer: { effectAllowed: '', dropEffect: '', setData: vi.fn(), ...overrides.dataTransfer },
    preventDefault: vi.fn(),
    stopPropagation: vi.fn(),
    ...overrides,
  };
}

beforeEach(() => {
  folderMoveStore.set(emptyFolderMove());
  state.workspaces.current = 'ws-1';
  vi.mocked(api.moveFolder).mockReset();
  vi.mocked(refreshNotesIndex).mockReset().mockResolvedValue(true);
});

describe('folderAcceptsFolderDrop', () => {
  it('takes another folder and refuses itself, its subtree, or an empty key', () => {
    expect(folderAcceptsFolderDrop('plans', 'p1/archive')).toBe(true);
    expect(folderAcceptsFolderDrop('plans', 'p1/plans')).toBe(false);
    expect(folderAcceptsFolderDrop('plans', 'p1/plans/inner')).toBe(false);
    expect(folderAcceptsFolderDrop('plans', '')).toBe(false);
  });
});

describe('drag state', () => {
  it('startFolderMove records the folder and its node, endFolderMove clears it', () => {
    startFolderMove('p1/plans', dragEvent());
    expect(folderMoveStore.getSnapshot()).toEqual({ path: 'plans', fromKey: 'p1/plans', over: '' });

    endFolderMove();
    expect(folderMoveStore.getSnapshot()).toEqual(emptyFolderMove());
  });

  it('startFolderMove sets the drag payload', () => {
    const event = dragEvent();
    startFolderMove('p1/plans', event);
    expect(event.dataTransfer.effectAllowed).toBe('move');
    expect(event.dataTransfer.setData).toHaveBeenCalledWith('text/plain', 'plans');
  });
});

describe('overFolderTarget', () => {
  it('highlights a folder that can take the dragged folder', () => {
    startFolderMove('p1/plans', dragEvent());
    const event = dragEvent();
    overFolderTarget('p1/archive', event);
    expect(event.preventDefault).toHaveBeenCalled();
    expect(event.stopPropagation).toHaveBeenCalled();
    expect(event.dataTransfer.dropEffect).toBe('move');
    expect(folderMoveStore.getSnapshot().over).toBe('p1/archive');
  });

  it('ignores itself and its own subtree without stopping the drag', () => {
    startFolderMove('p1/plans', dragEvent());
    const self = dragEvent();
    overFolderTarget('p1/plans', self);
    expect(self.preventDefault).not.toHaveBeenCalled();
    const child = dragEvent();
    overFolderTarget('p1/plans/inner', child);
    expect(child.preventDefault).not.toHaveBeenCalled();
    expect(folderMoveStore.getSnapshot().over).toBe('');
  });
});

describe('dropFolderOn', () => {
  it('moves through the backend and refreshes the index', async () => {
    vi.mocked(api.moveFolder).mockResolvedValue({ ok: true });
    startFolderMove('p1/plans', dragEvent());
    await dropFolderOn('p1/archive', dragEvent());
    expect(api.moveFolder).toHaveBeenCalledWith({
      workspace_id: 'ws-1',
      project_id: 'p1',
      path: 'plans',
      target: 'archive',
    });
    expect(refreshNotesIndex).toHaveBeenCalledTimes(1);
    expect(folderMoveStore.getSnapshot()).toEqual(emptyFolderMove());
  });

  it('drops on the sidebar background for the project root', async () => {
    vi.mocked(api.moveFolder).mockResolvedValue({ ok: true });
    startFolderMove('p1/deep/nested', dragEvent());
    await dropFolderOnRoot(dragEvent());
    expect(api.moveFolder).toHaveBeenCalledWith({
      workspace_id: 'ws-1',
      project_id: 'p1',
      path: 'deep/nested',
      target: '',
    });
    expect(refreshNotesIndex).toHaveBeenCalledTimes(1);
    expect(folderMoveStore.getSnapshot()).toEqual(emptyFolderMove());
  });

  it('keeps everything in place and says so when the backend refuses', async () => {
    vi.mocked(api.moveFolder).mockRejectedValue(new Error('Folder already exists'));
    const alert = vi.fn();
    vi.stubGlobal('alert', alert);
    startFolderMove('p1/plans', dragEvent());
    await dropFolderOn('p1/archive', dragEvent());
    expect(api.moveFolder).toHaveBeenCalledTimes(1);
    expect(refreshNotesIndex).not.toHaveBeenCalled();
    expect(alert).toHaveBeenCalledWith('Could not move the folder: Folder already exists');
    vi.unstubAllGlobals();
  });
});
