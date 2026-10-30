import { beforeEach, describe, expect, it, vi } from 'vitest';

const projects = { ws: [] };
let fetchError = '';

vi.mock('../../frontend/src/host/api.ts', () => ({
  fetchProjects: async (ws) => {
    if (fetchError) return { error: fetchError };
    if (!(ws in projects)) throw new Error(`unknown workspace ${ws}`);
    return { projects: projects[ws] };
  },
}));

const { describeWorkspaceDelete, summarizeWorkspaceDeleteCounts } = await import(
  '../../frontend/src/app-shell/commands/settings/notes-workspaces-delete.ts'
);

function row(id, name, extra = {}) {
  return {
    project_id: id,
    project_name: name,
    is_default: extra.is_default ?? id === 'inbox',
    doc_count: extra.doc_count ?? 0,
  };
}

describe('describeWorkspaceDelete', () => {
  it('uses singular and plural for zero, one and many documents', () => {
    expect(describeWorkspaceDelete('Work', 0, 1, 'Default', 'Inbox')).toBe(
      '“Work” has no documents. It will be deleted with its 1 project. This cannot be undone.',
    );
    expect(describeWorkspaceDelete('Work', 1, 2, 'Default', 'Inbox')).toBe(
      '1 document in “Work” will be moved to “Inbox” in “Default”, then “Work” and its 2 projects are deleted. This cannot be undone.',
    );
    expect(describeWorkspaceDelete('Work', 3, 1, 'Home', 'Catch-all')).toBe(
      '3 documents in “Work” will be moved to “Catch-all” in “Home”, then “Work” and its 1 project are deleted. This cannot be undone.',
    );
  });
});

describe('summarizeWorkspaceDeleteCounts', () => {
  beforeEach(() => {
    fetchError = '';
    projects.ws = [row('inbox', 'Inbox', { doc_count: 2, is_default: true }), row('plans', 'Plans', { doc_count: 3 })];
  });

  it('adds doc_count and names the inbox', async () => {
    await expect(summarizeWorkspaceDeleteCounts('ws')).resolves.toEqual({
      docCount: 5,
      projectCount: 2,
      inboxName: 'Inbox',
    });
  });

  it('counts a workspace that only has its inbox', async () => {
    projects.ws = [row('inbox', 'Catch-all', { doc_count: 0, is_default: true })];
    await expect(summarizeWorkspaceDeleteCounts('ws')).resolves.toEqual({
      docCount: 0,
      projectCount: 1,
      inboxName: 'Catch-all',
    });
  });

  it('throws when fetchProjects reports an error instead of returning zero', async () => {
    fetchError = 'down';
    await expect(summarizeWorkspaceDeleteCounts('ws')).rejects.toThrow('down');
  });

  it('throws when the payload has no projects list', async () => {
    projects.missing = undefined;
    await expect(summarizeWorkspaceDeleteCounts('missing')).rejects.toThrow('Could not load projects');
  });

  it('throws when the inbox row is missing', async () => {
    projects.ws = [row('plans', 'Plans', { doc_count: 1, is_default: false })];
    await expect(summarizeWorkspaceDeleteCounts('ws')).rejects.toThrow('Inbox missing');
  });
});
