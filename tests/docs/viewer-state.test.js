// @vitest-environment jsdom
import { beforeEach, describe, expect, it, vi } from 'vitest';

const saved = [];
let remembered = {};
let hold = null;

vi.mock('../../frontend/src/host/api.ts', () => ({
  fetchViewerState: async (ws) => {
    if (hold) await hold;
    return { id: remembered[ws] ?? '' };
  },
  saveViewerState: async (ws, id) => {
    saved.push([ws, id]);
    remembered[ws] = id;
    return { id };
  },
}));

const { state } = await import('../../frontend/src/host/state.ts');
const viewerState = await import('../../frontend/src/notes/commands/viewer-state.ts');

const opened = [];
const onOpen = (event) => opened.push(event.detail.id);

function setup({ view = 'directory', hash = '#/spark', ws = 'default' } = {}) {
  state.workspaces.current = ws;
  state.workspaces.list = [
    { id: 'default', name: 'Default', view_mode: view, is_default: true },
    { id: 'ws2', name: 'Wiki', view_mode: view, is_default: false },
  ];
  state.index.data = { d1: { id: 'd1', _id: 'd1' } };
  window.location.hash = hash;
}

describe('Open-document memory', () => {
  beforeEach(() => {
    saved.length = 0;
    opened.length = 0;
    remembered = {};
    hold = null;
    document.removeEventListener('cta:open-entry', onOpen);
    document.addEventListener('cta:open-entry', onOpen);
    setup();
  });

  it('remembers per workspace and forgets when the viewer is dismissed', async () => {
    viewerState.rememberOpenDoc('d1');
    state.workspaces.current = 'ws2';
    viewerState.forgetOpenDoc();
    await Promise.resolve();
    expect(saved).toEqual([['default', 'd1'], ['ws2', '']]);
  });

  it('reopens the remembered document on the list page of a directory workspace', async () => {
    remembered.default = 'd1';
    await viewerState.restoreOpenDoc();
    expect(opened).toEqual(['d1']);
  });

  it('does nothing in a timeline workspace, off the list route, or when a note is already named', async () => {
    remembered.default = 'd1';
    setup({ view: 'timeline' });
    await viewerState.restoreOpenDoc();
    setup({ hash: '#/home' });
    await viewerState.restoreOpenDoc();
    setup({ hash: '#/spark?date=2026-01-01&note=other' });
    await viewerState.restoreOpenDoc();
    expect(opened).toEqual([]);
    expect(saved).toEqual([]);
  });

  it('forgets a document that is gone (deleted or now hidden) instead of opening it', async () => {
    remembered.default = 'gone';
    await viewerState.restoreOpenDoc();
    expect(opened).toEqual([]);
    expect(saved).toEqual([['default', '']]);
  });

  it('does not open anything when the workspace changed while the cache was read', async () => {
    remembered.default = 'd1';
    let release;
    hold = new Promise((r) => (release = r));
    const pending = viewerState.restoreOpenDoc();
    state.workspaces.current = 'ws2';
    release();
    await pending;
    expect(opened).toEqual([]);
  });
});
