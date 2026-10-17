// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';

let backend = {};
const saved = [];
let hash = '#/spark';

vi.mock('../../frontend/src/notes/commands/cards.ts', () => ({ loadTitles: vi.fn() }));
vi.mock('../../frontend/src/router/index.ts', () => ({
  parseHash: vi.fn(() => {
    const [, query = ''] = hash.split('?');
    return { name: 'spark', params: Object.fromEntries(new URLSearchParams(query)) };
  }),
  navigateToDateList: vi.fn(),
}));
vi.mock('../../frontend/src/host/api.ts', () => ({
  fetchViewerState: async (ws) => backend[ws] ?? { version: 2, project: '', projects: {} },
  saveViewerState: async (ws, patch) => {
    saved.push([ws, patch]);
    const mem = (backend[ws] ??= { version: 2, project: '', projects: {} });
    mem.project = patch.project;
    if (!patch.project) return mem;
    const entry = { node: '', doc: '', date: '', ...mem.projects[patch.project], ...patch };
    delete entry.project;
    mem.projects[patch.project] = entry;
    return mem;
  },
}));

const { state } = await import('../../frontend/src/host/state.ts');
const { applyListFilters, buildGroups, selectDate, selectProject } = await import(
  '../../frontend/src/notes/commands/sidebar.ts'
);
const { dirKeyOf, dirTreeStore, resetDirTree } = await import('../../frontend/src/notes/state/dir-tree.ts');
const { enterProject, enterWorkspace } = await import('../../frontend/src/notes/commands/restore.ts');
const memory = await import('../../frontend/src/notes/commands/viewer-state.ts');
const { resolveNotesListDate } = await import('../../frontend/src/notes/commands/viewer/outlet.ts');

const data = {
  d1: { id: 'd1', _id: 'd1', project_id: 'ai', raw_path: 'plans/one.md', created_at: '202605011200' },
  d2: { id: 'd2', _id: 'd2', project_id: 'ai', raw_path: 'two.md', created_at: '202605021200' },
  d3: { id: 'd3', _id: 'd3', project_id: 'ops', raw_path: 'run/book/three.md', created_at: '202604011200' },
  d4: { id: 'd4', _id: 'd4', project_id: 'ops', raw_path: 'four.md', created_at: '202603011200' },
};
const titles = { ai: 'AI', ops: 'Ops', empty: 'Empty' };
const opened = [];
const onOpen = (event) => opened.push(event.detail.id);

async function enter(view, { ws = 'default', order = ['ai', 'ops', 'empty'], recorded } = {}) {
  await memory.flushViewerWrites();
  state.workspaces.current = ws;
  state.workspaces.list = [{ id: ws, name: ws, view_mode: view, is_default: true }];
  state.index.data = data;
  state.index.topicTitles = titles;
  state.index.topicOrder = order;
  state.index.groupedByDate = buildGroups(data);
  state.ui.activeTopic = null;
  state.ui.activeDate = null;
  resetDirTree();
  applyListFilters();
  if (recorded) backend[ws] = recorded;
  else delete backend[ws];
  await memory.loadViewerMemory(ws);
}

const rec = (project, projects = {}) => ({ version: 2, project, projects });
const part = (extra) => ({ node: '', doc: '', date: '', ...extra });

beforeEach(() => {
  backend = {};
  saved.length = 0;
  opened.length = 0;
  hash = '#/spark';
  document.removeEventListener('cta:open-entry', onOpen);
  document.addEventListener('cta:open-entry', onOpen);
});

describe('directory view: entering a Project restores its record', () => {
  it('locates the remembered folder and opens the remembered document', async () => {
    await enter('directory', { recorded: rec('ai', { ai: part({ node: 'ai/plans', doc: 'd1' }) }) });
    enterProject('ai');
    expect(state.ui.activeTopic).toBe('ai');
    expect(state.ui.activeDate).toBe(dirKeyOf('ai/plans'));
    expect(dirTreeStore.getSnapshot().expanded).toEqual(['ai', 'ai/plans']);
    expect(state.index.filteredGroups[0].entries.map((e) => e.id)).toEqual(['d1']);
    expect(opened).toEqual(['d1']);
  });

  it('a document that is gone or belongs to another project only locates the folder', async () => {
    await enter('directory', { recorded: rec('ai', { ai: part({ node: 'ai/plans', doc: 'gone' }) }) });
    enterProject('ai');
    expect(state.ui.activeDate).toBe(dirKeyOf('ai/plans'));
    backend.default.projects.ai.doc = 'd3';
    await memory.loadViewerMemory('default');
    enterProject('ai');
    expect(opened).toEqual([]);
  });

  it('a folder that no longer exists is treated as no record: the right pane stays blank', async () => {
    await enter('directory', { recorded: rec('ai', { ai: part({ node: 'ai/removed', doc: 'd1' }) }) });
    enterProject('ai');
    expect(state.ui.activeTopic).toBe('ai');
    expect(state.ui.activeDate).toBeNull();
    expect(opened).toEqual([]);
  });

  it('with no record the right pane is blank and never falls back to a timeline day', async () => {
    await enter('directory');
    enterProject('ops');
    expect(state.ui.activeTopic).toBe('ops');
    expect(state.ui.activeDate).toBeNull();
    expect(state.index.filteredGroups).toEqual([]);
    expect(resolveNotesListDate(state.ui.activeDate, null)).toEqual({ date: '', group: null });
    expect(opened).toEqual([]);
  });

  it('does not open a document while another note is already shown', async () => {
    hash = '#/spark?note=other';
    await enter('directory', { recorded: rec('ai', { ai: part({ node: 'ai/plans', doc: 'd1' }) }) });
    enterProject('ai');
    expect(opened).toEqual([]);
  });

  it('each project keeps its own record across switches', async () => {
    await enter('directory');
    enterProject('ai');
    selectDate(dirKeyOf('ai/plans'));
    enterProject('ops');
    selectDate(dirKeyOf('ops/run/book'));
    enterProject('ai');
    expect(state.ui.activeDate).toBe(dirKeyOf('ai/plans'));
    enterProject('ops');
    expect(state.ui.activeDate).toBe(dirKeyOf('ops/run/book'));
    enterProject('empty');
    expect(state.ui.activeDate).toBeNull();
  });
});

describe('timeline: entering a Project restores its day', () => {
  it('goes back to the remembered day, or the latest when there is none or it has no data', async () => {
    await enter('timeline', { recorded: rec('ops', { ops: part({ date: '20260401' }) }) });
    enterProject('ops');
    expect(state.ui.activeDate).toBe('20260401');
    await enter('timeline', { recorded: rec('ops', { ops: part({ date: '20250101' }) }) });
    enterProject('ops');
    expect(state.ui.activeDate).toBe('20260401');
    await enter('timeline');
    enterProject('ai');
    expect(state.ui.activeDate).toBe('20260502');
  });

  it('does not open a document: only the day is remembered', async () => {
    await enter('timeline', { recorded: rec('ai', { ai: part({ date: '20260501', doc: 'd1' }) }) });
    enterProject('ai');
    expect(state.ui.activeDate).toBe('20260501');
    expect(opened).toEqual([]);
  });
});

describe('entering a workspace', () => {
  it('goes to the last selected project', async () => {
    await enter('directory', { recorded: rec('ops') });
    enterWorkspace();
    expect(state.ui.activeTopic).toBe('ops');
  });

  it('without a record, or when the project is gone, takes the first project in order', async () => {
    await enter('directory', { order: ['ops', 'ai', 'empty'] });
    enterWorkspace();
    expect(state.ui.activeTopic).toBe('ops');
    await enter('directory', { recorded: rec('disabled', { disabled: part({ node: 'disabled/x' }) }) });
    enterWorkspace();
    expect(state.ui.activeTopic).toBe('ai');
    expect(state.ui.activeDate).toBeNull();
  });

  it('a remembered All in a directory workspace counts as no record', async () => {
    await enter('directory', { recorded: rec('', { ops: part({ node: 'ops/run' }) }) });
    enterWorkspace();
    expect(state.ui.activeTopic).toBe('ai');
    expect(state.ui.activeDate).toBeNull();
  });

  it('timeline: All is remembered too, and no record means All at the latest day', async () => {
    await enter('timeline');
    enterWorkspace();
    expect(state.ui.activeTopic).toBeNull();
    expect(state.ui.activeDate).toBe('20260502');
    await enter('timeline', { recorded: rec('ai', { ai: part({ date: '20260501' }) }) });
    enterWorkspace();
    expect(state.ui.activeTopic).toBe('ai');
    expect(state.ui.activeDate).toBe('20260501');
    await enter('timeline', { recorded: rec('') });
    enterWorkspace();
    expect(state.ui.activeTopic).toBeNull();
  });

  it('with no projects at all the directory view has nothing selected', async () => {
    await enter('directory', { order: [] });
    state.index.topicTitles = {};
    enterWorkspace();
    expect(state.ui.activeTopic).toBeNull();
  });

  it('each workspace keeps its own record', async () => {
    await enter('directory', { ws: 'ws2', recorded: rec('ops') });
    await enter('directory', { ws: 'default', recorded: rec('ai') });
    await memory.loadViewerMemory('ws2');
    state.workspaces.current = 'ws2';
    expect(memory.remembered().project).toBe('ops');
    state.workspaces.current = 'default';
    expect(memory.remembered().project).toBe('ai');
  });
});

describe('writing the record', () => {
  const last = () => saved[saved.length - 1];
  const flush = () => memory.flushViewerWrites();

  it('records the project, the folder, the document and the day as they are chosen', async () => {
    await enter('directory');
    selectProject('ai');
    await flush();
    expect(last()).toEqual(['default', { project: 'ai' }]);
    selectDate(dirKeyOf('ai/plans'));
    await flush();
    expect(last()).toEqual(['default', { project: 'ai', node: 'ai/plans' }]);
    memory.rememberOpenDoc(data.d1, 'd1');
    await flush();
    expect(last()).toEqual(['default', { project: 'ai', node: 'ai/plans', doc: 'd1' }]);
    selectDate(dirKeyOf('ai'));
    await flush();
    expect(last()).toEqual(['default', { project: 'ai', node: 'ai', doc: '' }]);
    memory.rememberOpenDoc(data.d2, 'd2');
    memory.forgetOpenDoc();
    await flush();
    expect(last()).toEqual(['default', { project: 'ai', doc: '' }]);
    await enter('timeline');
    enterProject('ops');
    selectDate('20260401');
    await flush();
    expect(last()).toEqual(['default', { project: 'ops', date: '20260401' }]);
  });

  it('restoring does not overwrite the record with its own selections', async () => {
    await enter('directory', { recorded: rec('ai', { ai: part({ node: 'ai/plans', doc: 'd1' }) }) });
    enterProject('ai');
    await memory.flushViewerWrites();
    expect(saved).toEqual([['default', { project: 'ai' }]]);
    expect(backend.default.projects.ai).toEqual(part({ node: 'ai/plans', doc: 'd1' }));
  });

  it('does not record a document in the timeline, or one without a project', async () => {
    await enter('timeline');
    memory.rememberOpenDoc(data.d1, 'd1');
    await enter('directory');
    memory.rememberOpenDoc({ raw_path: 'x.md' }, 'x');
    await memory.flushViewerWrites();
    expect(saved).toEqual([]);
  });
});
