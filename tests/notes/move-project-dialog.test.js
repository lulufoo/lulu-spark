// @vitest-environment jsdom
import { describe, it, expect, beforeEach, vi } from 'vitest';
import { state } from '../../frontend/src/host/state.ts';
import * as api from '../../frontend/src/host/api.ts';
import {
  closeMoveProjectDialog,
  doMoveProject,
  openMoveProjectDialog,
  selectMoveWorkspace,
} from '../../frontend/src/notes/commands/move-project-dialog.ts';
import { moveProjectViewStore } from '../../frontend/src/notes/state/move-project.ts';

vi.mock('../../frontend/src/host/api.ts', () => ({
  fetchWorkspaces: vi.fn(),
  fetchProjects: vi.fn(),
  moveToProject: vi.fn(),
}));

const CURRENT = 'ws-current';
const OTHER = 'ws-other';
const DOC = 'a'.repeat(32);

beforeEach(() => {
  closeMoveProjectDialog();
  state.workspaces.current = CURRENT;
  vi.mocked(api.fetchWorkspaces).mockResolvedValue({
    workspaces: [
      { workspace_id: CURRENT, workspace_name: 'Default' },
      { workspace_id: OTHER, workspace_name: 'Dest' },
    ],
  });
  vi.mocked(api.fetchProjects).mockImplementation(async (workspaceId) => {
    if (workspaceId === CURRENT) {
      return {
        projects: [
          { project_id: 'inbox-a', project_name: 'Inbox', is_default: true },
          { project_id: 'plans', project_name: 'Plans' },
        ],
      };
    }
    return {
      projects: [{ project_id: 'shelf', project_name: 'Shelf' }],
    };
  });
  vi.mocked(api.moveToProject).mockResolvedValue({ ok: true });
});

describe('move-project two-level picker', () => {
  it('defaults to the current workspace and hides the current project', async () => {
    await openMoveProjectDialog({
      id: DOC,
      workspace_id: CURRENT,
      project_id: 'inbox-a',
    });
    const view = moveProjectViewStore.getSnapshot();
    expect(view.selectedWorkspaceId).toBe(CURRENT);
    expect(view.workspaces.map((w) => w.workspace_id)).toEqual([CURRENT, OTHER]);
    expect(view.items.map((p) => p.project_id)).toEqual(['plans']);
    expect(api.fetchProjects).toHaveBeenCalledWith(CURRENT);
  });

  it('loads the other workspace projects, then moves with target_workspace_id', async () => {
    await openMoveProjectDialog({
      id: DOC,
      workspace_id: CURRENT,
      project_id: 'inbox-a',
    });
    await selectMoveWorkspace(OTHER);
    expect(moveProjectViewStore.getSnapshot().selectedWorkspaceId).toBe(OTHER);
    expect(moveProjectViewStore.getSnapshot().items.map((p) => p.project_id)).toEqual(['shelf']);
    await doMoveProject('shelf');
    expect(api.moveToProject).toHaveBeenCalledWith(DOC, 'shelf', OTHER);
  });
});
