// @vitest-environment jsdom
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { beforeEach, describe, expect, it, vi } from 'vitest';

const rules = { default: [], ws2: [] };
const calls = [];
let nextId = 1;
let hold = null;

const list = (ws) => ({ patterns: rules[ws].map((r) => ({ ...r })) });

vi.mock('../../frontend/src/host/api.ts', () => ({
  fetchHidePatterns: async (ws) => {
    if (hold && hold.ws === ws) await hold.promise;
    return list(ws);
  },
  addHidePattern: async (ws, pattern) => {
    calls.push(['add', ws, pattern]);
    if (pattern === 'dup') return { error: 'pattern already exists' };
    rules[ws].push({ id: `r${nextId++}`, pattern });
    return list(ws);
  },
  updateHidePattern: async (ws, id, pattern) => {
    calls.push(['update', ws, id, pattern]);
    rules[ws] = rules[ws].map((r) => (r.id === id ? { id, pattern } : r));
    return list(ws);
  },
  removeHidePattern: async (ws, id) => {
    calls.push(['remove', ws, id]);
    rules[ws] = rules[ws].filter((r) => r.id !== id);
    return list(ws);
  },
}));

const { state } = await import('../../frontend/src/host/state.ts');
const commands = await import('../../frontend/src/app-shell/commands/settings/hide-patterns.ts');
const { hidePatternsStore, emptyHidePatternsSnap } = await import(
  '../../frontend/src/app-shell/state/settings/hide-patterns.ts'
);
const { HiddenFilesTab } = await import('../../frontend/src/app-shell/ui/settings/hidden-files-tab.tsx');

const snap = () => hidePatternsStore.getSnapshot();

describe('Hidden files rules per workspace', () => {
  const reloads = vi.fn();

  beforeEach(() => {
    calls.length = 0;
    nextId = 1;
    hold = null;
    rules.default = [{ id: 'a', pattern: '^\\.git$' }];
    rules.ws2 = [{ id: 'b', pattern: '\\.tmp$' }];
    hidePatternsStore.set(emptyHidePatternsSnap());
    state.workspaces.current = 'default';
    reloads.mockClear();
    document.removeEventListener('cta:reload', reloads);
    document.addEventListener('cta:reload', reloads);
  });

  it('shows the rules of the selected workspace only', async () => {
    await commands.selectHidePatternsWorkspace('ws2');
    expect(snap().workspaceId).toBe('ws2');
    expect(snap().rows).toEqual([{ id: 'b', pattern: '\\.tmp$' }]);
  });

  it('acts on the selected workspace and reloads the page only when it is the shown one', async () => {
    await commands.selectHidePatternsWorkspace('ws2');
    commands.setHidePatternAddValue('\\.bak$');
    await commands.addHidePatternRow();
    expect(calls).toEqual([['add', 'ws2', '\\.bak$']]);
    expect(snap().addValue).toBe('');
    expect(snap().rows.map((r) => r.pattern)).toEqual(['\\.tmp$', '\\.bak$']);
    expect(reloads).not.toHaveBeenCalled();

    await commands.selectHidePatternsWorkspace('default');
    await commands.removeHidePatternRow('a');
    expect(calls.at(-1)).toEqual(['remove', 'default', 'a']);
    expect(snap().rows).toEqual([]);
    expect(reloads).toHaveBeenCalledTimes(1);
  });

  it('edits a row in place and saves the trimmed value', async () => {
    await commands.selectHidePatternsWorkspace('default');
    commands.startHidePatternEdit('a');
    expect(snap().draft).toBe('^\\.git$');
    commands.setHidePatternDraft('  ^\\.svn$ ');
    await commands.saveHidePatternEdit();
    expect(calls).toEqual([['update', 'default', 'a', '^\\.svn$']]);
    expect(snap()).toMatchObject({ editingId: '', draft: '' });
  });

  it('rejects an invalid regex before calling the host, and keeps the input', async () => {
    await commands.selectHidePatternsWorkspace('default');
    commands.setHidePatternAddValue('(');
    await commands.addHidePatternRow();
    expect(calls).toEqual([]);
    expect(snap().error).toMatch(/^Invalid regex:/);
    expect(snap().addValue).toBe('(');
    expect(reloads).not.toHaveBeenCalled();
  });

  it('shows a host refusal and keeps the typed rule', async () => {
    await commands.selectHidePatternsWorkspace('default');
    commands.setHidePatternAddValue('dup');
    await commands.addHidePatternRow();
    expect(snap().error).toBe('pattern already exists');
    expect(snap().addValue).toBe('dup');
    expect(reloads).not.toHaveBeenCalled();
  });

  it('drops a late answer for a workspace that is no longer selected', async () => {
    let release;
    hold = { ws: 'default', promise: new Promise((r) => (release = r)) };
    const first = commands.selectHidePatternsWorkspace('default');
    await commands.selectHidePatternsWorkspace('ws2');
    release();
    await first;
    expect(snap().workspaceId).toBe('ws2');
    expect(snap().rows.map((r) => r.id)).toEqual(['b']);
  });

  it('paints rows, an Add input, and the example hint', async () => {
    await commands.selectHidePatternsWorkspace('default');
    const host = document.createElement('div');
    document.body.appendChild(host);
    const root = createRoot(host);
    act(() => root.render(createElement(HiddenFilesTab)));
    expect(host.querySelector('#settings-tab-workspace-hidden').dataset.tab).toBe('hidden');
    expect([...host.querySelectorAll('.kb-hide-row-text')].map((b) => b.textContent)).toEqual(['^\\.git$']);
    expect(host.querySelector('#settings-kb-hide-add')).not.toBeNull();
    expect(host.querySelector('#btn-settings-kb-hide-add').textContent).toBe('Add');
    expect(host.textContent).toContain('\\.xxx$');
    act(() => root.unmount());
  });
});
