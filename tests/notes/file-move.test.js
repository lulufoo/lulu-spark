// @vitest-environment jsdom
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { state } from '../../frontend/src/host/state.ts';
import * as api from '../../frontend/src/host/api.ts';
import {
  dropFileOn,
  endFileMove,
  folderAcceptsDrop,
  leaveFolder,
  overFolder,
  startFileMove,
} from '../../frontend/src/notes/commands/file-move.ts';
import { emptyFileMove, fileMoveStore } from '../../frontend/src/notes/state/file-move.ts';
import { refreshNotesIndex } from '../../frontend/src/notes/commands/reload-index.ts';

vi.mock('../../frontend/src/host/api.ts', () => ({
  moveDocDir: vi.fn(),
}));

vi.mock('../../frontend/src/notes/commands/reload-index.ts', () => ({
  refreshNotesIndex: vi.fn(),
}));

const DOC = 'a'.repeat(32);

function dragEvent(overrides = {}) {
  return {
    dataTransfer: { effectAllowed: '', dropEffect: '', setData: vi.fn(), ...overrides.dataTransfer },
    preventDefault: vi.fn(),
    stopPropagation: vi.fn(),
    ...overrides,
  };
}

beforeEach(() => {
  fileMoveStore.set(emptyFileMove());
  state.workspaces.current = 'ws-1';
  state.index.data = { [DOC]: { id: DOC, project_id: 'p1', raw_path: 'plans/doc.md' } };
  state.viewer.entry = null;
  vi.mocked(api.moveDocDir).mockReset();
  vi.mocked(refreshNotesIndex).mockReset().mockResolvedValue(true);
});

describe('folderAcceptsDrop', () => {
  it('takes another folder of the same project and refuses the current one or another project', () => {
    expect(folderAcceptsDrop('p1/plans', 'p1/archive')).toBe(true);
    expect(folderAcceptsDrop('p1/plans', 'p1/plans')).toBe(false);
    expect(folderAcceptsDrop('p1/plans', 'p2/archive')).toBe(false);
    expect(folderAcceptsDrop('', 'p1/archive')).toBe(false);
    expect(folderAcceptsDrop('p1/plans', '')).toBe(false);
  });
});

describe('drag state', () => {
  it('startFileMove records the document and its node, endFileMove clears it', () => {
    startFileMove(DOC, state.index.data[DOC], dragEvent());
    expect(fileMoveStore.getSnapshot()).toEqual({ id: DOC, fromKey: 'p1/plans', over: '' });

    endFileMove();
    expect(fileMoveStore.getSnapshot()).toEqual(emptyFileMove());
  });

  it('startFileMove sets the drag payload', () => {
    const event = dragEvent();
    startFileMove(DOC, state.index.data[DOC], event);
    expect(event.dataTransfer.effectAllowed).toBe('move');
    expect(event.dataTransfer.setData).toHaveBeenCalledWith('text/plain', 'plans/doc.md');
  });
});

describe('overFolder', () => {
  it('highlights a folder that can take the drop', () => {
    startFileMove(DOC, state.index.data[DOC], dragEvent());
    const event = dragEvent();
    overFolder('p1/archive', event);
    expect(event.preventDefault).toHaveBeenCalled();
    expect(event.stopPropagation).toHaveBeenCalled();
    expect(event.dataTransfer.dropEffect).toBe('move');
    expect(fileMoveStore.getSnapshot().over).toBe('p1/archive');
  });

  it('ignores the current folder and another project without stopping the drag', () => {
    startFileMove(DOC, state.index.data[DOC], dragEvent());
    const same = dragEvent();
    overFolder('p1/plans', same);
    expect(same.preventDefault).not.toHaveBeenCalled();
    expect(fileMoveStore.getSnapshot().over).toBe('');

    const other = dragEvent();
    overFolder('p2/archive', other);
    expect(other.preventDefault).not.toHaveBeenCalled();
    expect(fileMoveStore.getSnapshot().over).toBe('');
  });

  it('leaveFolder clears the highlight of that row only', () => {
    startFileMove(DOC, state.index.data[DOC], dragEvent());
    overFolder('p1/archive', dragEvent());
    leaveFolder('p1/plans');
    expect(fileMoveStore.getSnapshot().over).toBe('p1/archive');
    leaveFolder('p1/archive');
    expect(fileMoveStore.getSnapshot().over).toBe('');
  });
});

describe('dropFileOn', () => {
  it('moves through the backend and refreshes the index', async () => {
    vi.mocked(api.moveDocDir).mockResolvedValue({ ok: true });
    startFileMove(DOC, state.index.data[DOC], dragEvent());
    const event = dragEvent();
    await dropFileOn('p1/archive', event);
    expect(event.preventDefault).toHaveBeenCalled();
    expect(api.moveDocDir).toHaveBeenCalledWith({
      workspace_id: 'ws-1',
      project_id: 'p1',
      id: DOC,
      target: 'archive',
    });
    expect(refreshNotesIndex).toHaveBeenCalledTimes(1);
    expect(fileMoveStore.getSnapshot()).toEqual(emptyFileMove());
  });

  it('strips the project prefix from nested folder keys', async () => {
    vi.mocked(api.moveDocDir).mockResolvedValue({ ok: true });
    state.index.data[DOC].raw_path = 'plans/doc.md';
    startFileMove(DOC, state.index.data[DOC], dragEvent());
    await dropFileOn('p1/deep/nested', dragEvent());
    expect(api.moveDocDir).toHaveBeenCalledWith(
      expect.objectContaining({ target: 'deep/nested' }),
    );
  });

  it('does nothing on an invalid target', async () => {
    startFileMove(DOC, state.index.data[DOC], dragEvent());
    await dropFileOn('p1/plans', dragEvent());
    expect(api.moveDocDir).not.toHaveBeenCalled();
    expect(refreshNotesIndex).not.toHaveBeenCalled();
    expect(fileMoveStore.getSnapshot()).toEqual(emptyFileMove());
  });

  it('keeps everything in place and says so when the backend refuses', async () => {
    vi.mocked(api.moveDocDir).mockRejectedValue(new Error('File already exists'));
    const alert = vi.fn();
    vi.stubGlobal('alert', alert);
    startFileMove(DOC, state.index.data[DOC], dragEvent());
    await dropFileOn('p1/archive', dragEvent());
    expect(api.moveDocDir).toHaveBeenCalledTimes(1);
    expect(refreshNotesIndex).not.toHaveBeenCalled();
    expect(alert).toHaveBeenCalledWith('Could not move the file: File already exists');
    vi.unstubAllGlobals();
  });
});
