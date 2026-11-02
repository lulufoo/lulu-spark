import { beforeEach, describe, expect, it, vi } from 'vitest';

const fetched = [];
let localEntries = {};
let gate = null;

vi.mock('../../frontend/src/host/api.ts', () => ({
  fetchIndex: async (workspaceId) => {
    fetched.push(workspaceId);
    if (gate) await gate;
    return { entries: localEntries };
  },
}));

const { state } = await import('../../frontend/src/host/state.ts');
const { projectListLines, reloadProjectListIfStale, sameProjectList } = await import(
  '../../frontend/src/notes/commands/project-list.ts'
);

const entry = (id, projectId, rawPath) => ({ id, project_id: projectId, raw_path: rawPath, _id: id });

describe('project list lines', () => {
  it('lists only that project, id and path, in order', () => {
    const lines = projectListLines(
      {
        b: entry('b', 'kotlin', 'b.md'),
        a: entry('a', 'kotlin', 'a.md'),
        c: entry('c', 'inbox', 'c.md'),
      },
      'kotlin',
    );
    expect(lines).toEqual(['a\ta.md', 'b\tb.md']);
    expect(sameProjectList(lines, ['a\ta.md', 'b\tb.md'])).toBe(true);
    expect(sameProjectList(lines, ['a\ta.md'])).toBe(false);
  });
});

describe('reloadProjectListIfStale', () => {
  beforeEach(() => {
    fetched.length = 0;
    localEntries = {};
    gate = null;
    state.workspaces.current = 'mobile';
    state.ui.activeTopic = 'kotlin';
    state.index.data = { old: entry('old', 'kotlin', 'old.md') };
    state.index.groupedByDate = [];
    state.index.filteredGroups = [];
  });

  it('leaves memory alone when the local list matches', async () => {
    localEntries = { old: { id: 'old', project_id: 'kotlin', raw_path: 'old.md' } };
    const before = state.index.data;
    await reloadProjectListIfStale('kotlin');
    expect(fetched).toEqual(['mobile']);
    expect(state.index.data).toBe(before);
  });

  it('replaces memory when the local list has different documents', async () => {
    localEntries = {
      a: { id: 'a', project_id: 'kotlin', raw_path: 'a.md' },
      b: { id: 'b', project_id: 'kotlin', raw_path: 'b.md' },
    };
    await reloadProjectListIfStale('kotlin');
    expect(Object.keys(state.index.data).sort()).toEqual(['a', 'b']);
  });

  it('does not apply a late answer after the user opened another project', async () => {
    let release;
    gate = new Promise((resolve) => {
      release = resolve;
    });
    localEntries = { a: { id: 'a', project_id: 'kotlin', raw_path: 'a.md' } };
    const running = reloadProjectListIfStale('kotlin');
    state.ui.activeTopic = 'inbox';
    release();
    await running;
    expect(state.index.data.old.project_id).toBe('kotlin');
  });
});
