// @vitest-environment jsdom
import { act, createElement } from 'react';
import { createRoot } from 'react-dom/client';
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest';

const calls = [];
let failOn = '';
let failMessage = '';

function reply(name, args) {
  calls.push([name, ...args]);
  if (failOn === name) {
    failOn = '';
    return { error: failMessage };
  }
  return { ok: true };
}

const projects = { default: [] };

vi.mock('../../frontend/src/host/api.ts', () => ({
  createWorkspace: async (...a) => reply('create-ws', a),
  renameWorkspace: async (...a) => reply('rename-ws', a),
  setWorkspaceViewMode: async (...a) => reply('view-ws', a),
  setWorkspaceIcon: async (...a) => reply('icon-ws', a),
  disableWorkspace: async (...a) => reply('disable-ws', a),
  enableWorkspace: async (...a) => reply('enable-ws', a),
  fetchDisabledWorkspaces: async () => ({ workspaces: [] }),
  fetchProjects: async (ws) => ({ projects: projects[ws] }),
  createProject: async (...a) => reply('create-p', a),
  updateProject: async (...a) => reply('update-p', a),
}));
vi.mock('../../frontend/src/notes/commands/workspace.ts', () => ({ loadWorkspaces: async () => {} }));
vi.mock('../../frontend/src/notes/commands/workspace-switch.ts', () => ({ applyViewModeChange: () => {} }));

const { state } = await import('../../frontend/src/host/state.ts');
const wsCommands = await import('../../frontend/src/app-shell/commands/settings/workspaces.ts');
const catCommands = await import('../../frontend/src/app-shell/commands/settings/notes-categories.ts');
const { workspacesPanelStore, emptyWorkspacesPanelSnap } = await import(
  '../../frontend/src/app-shell/state/settings/workspaces-panel.ts'
);
const { notesCategoriesStore, emptyNotesCategoriesSnap } = await import(
  '../../frontend/src/app-shell/state/settings/notes-categories.ts'
);
const { IconPicker } = await import('../../frontend/src/app-shell/ui/settings/icon-picker.tsx');
const { NotesWorkspacesEditHost } = await import('../../frontend/src/app-shell/ui/settings/notes-workspaces-edit.tsx');
const { NotesWorkspacesAddHost } = await import('../../frontend/src/app-shell/ui/settings/notes-workspaces-add.tsx');
const { NotesCategoriesEditHost } = await import('../../frontend/src/app-shell/ui/settings/notes-categories-edit.tsx');
const { NotesCategoriesAddHost } = await import('../../frontend/src/app-shell/ui/settings/notes-categories-add.tsx');
const { ICONS, DEFAULT_PROJECT_ICON_ID, getIcon } = await import('../../frontend/src/shared/icons/index.ts');

const wsSnap = () => workspacesPanelStore.getSnapshot();
const catSnap = () => notesCategoriesStore.getSnapshot();
const ICON_A = ICONS.find((i) => i.category === '00').id;
const ICON_B = ICONS.find((i) => i.category === '04').id;
const DIRECTION = ICONS.find((i) => i.category === '06').id;

let host;
let root;
function mount(component, props = {}) {
  host = document.createElement('div');
  document.body.appendChild(host);
  root = createRoot(host);
  act(() => root.render(createElement(component, props)));
}
afterEach(() => {
  if (root) act(() => root.unmount());
  host?.remove();
  root = null;
  host = null;
});

beforeEach(() => {
  calls.length = 0;
  failOn = '';
  workspacesPanelStore.set(emptyWorkspacesPanelSnap());
  notesCategoriesStore.set(emptyNotesCategoriesSnap());
  state.workspaces.current = 'default';
  state.workspaces.list = [
    { workspace_id: 'default', workspace_name: 'Default', view_mode: 'directory', is_default: true, icon: '' },
    { workspace_id: 'ws2', workspace_name: 'Wiki', view_mode: 'timeline', is_default: false, icon: ICON_A },
  ];
  projects.default = [
    { project_id: 'inbox', project_name: 'Inbox', description: '', icon: '', is_inbox: true },
    { project_id: 'plans', project_name: 'Plans', description: 'd', icon: ICON_A },
  ];
});

describe('IconPicker', () => {
  function mountPicker(props = {}) {
    const onChange = vi.fn();
    mount(IconPicker, {
      label: 'Test icon',
      value: '',
      defaultIcon: createElement('span', { 'data-testid': 'default-icon' }, 'D'),
      onChange,
      ...props,
    });
    return onChange;
  }
  const button = () => host.querySelector('.icon-picker-button');
  const panel = () => document.body.querySelector('.icon-picker-panel');
  const open = () => act(() => button().click());

  it('shows the default icon until one is chosen, then the chosen icon', () => {
    mountPicker();
    expect(button().querySelector('[data-testid="default-icon"]')).not.toBeNull();
    act(() => root.render(createElement(IconPicker, { label: 'Test icon', value: ICON_A, defaultIcon: null, onChange() {} })));
    expect(button().querySelector(`.icon-picker-main svg[data-icon-id="${ICON_A}"]`)).not.toBeNull();
  });

  it('carries a bottom-right pencil badge so it reads as editable, with or without a chosen icon', () => {
    mountPicker();
    const badge = () => button().querySelector('.icon-picker-edit-badge');
    expect(badge()?.querySelector('svg[data-icon-id="01yoo4"]')).not.toBeNull();
    act(() => root.render(createElement(IconPicker, { label: 'Test icon', value: ICON_A, defaultIcon: null, onChange() {} })));
    expect(badge()?.querySelector('svg[data-icon-id="01yoo4"]')).not.toBeNull();
    // The badge is decoration: the button keeps its accessible name and the badge is not a control.
    expect(button().getAttribute('aria-label')).toBe('Test icon');
    expect(badge().getAttribute('aria-hidden')).toBe('true');
    expect(badge().querySelector('button')).toBeNull();
  });

  it('falls back to the default for a well-formed id the module does not know', () => {
    mountPicker({ value: '00zzzz' });
    expect(button().querySelector('[data-testid="default-icon"]')).not.toBeNull();
  });

  it('opens a grid grouped by category without Directions, marking the chosen icon', () => {
    mountPicker({ value: ICON_B });
    expect(panel()).toBeNull();
    open();
    const names = [...panel().querySelectorAll('.icon-picker-category-title')].map((h) => h.textContent);
    expect(names).toEqual(['Shapes', 'Process', 'People', 'Content', 'Tech', 'Marks']);
    expect(panel().querySelectorAll('.icon-picker-cell')).toHaveLength(89);
    expect(panel().querySelector(`[data-icon-id="${DIRECTION}"]`)).toBeNull();
    expect(panel().querySelector(`.icon-picker-cell.is-selected`).dataset.iconId).toBe(ICON_B);
    expect(panel().querySelector('[data-icon-default="true"]').getAttribute('aria-pressed')).toBe('false');
  });

  it('choosing an icon reports it and closes the grid; Default reports an empty id', () => {
    const onChange = mountPicker({ value: ICON_B });
    open();
    act(() => panel().querySelector(`[data-icon-id="${ICON_A}"]`).click());
    expect(onChange).toHaveBeenLastCalledWith(ICON_A);
    expect(panel()).toBeNull();
    open();
    act(() => panel().querySelector('[data-icon-default="true"]').click());
    expect(onChange).toHaveBeenLastCalledWith('');
    expect(panel()).toBeNull();
  });

  it('Esc closes only the grid and does not reach a listener behind it', () => {
    mountPicker();
    const behind = vi.fn();
    window.addEventListener('keydown', behind);
    open();
    act(() => {
      window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }));
    });
    expect(panel()).toBeNull();
    expect(behind).not.toHaveBeenCalled();
    act(() => {
      window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }));
    });
    expect(behind).toHaveBeenCalledTimes(1);
    window.removeEventListener('keydown', behind);
  });

  it('a click outside closes the grid and is not passed on', () => {
    mountPicker();
    const behind = vi.fn();
    const outside = document.createElement('div');
    outside.addEventListener('click', behind);
    document.body.appendChild(outside);
    open();
    act(() => outside.click());
    expect(panel()).toBeNull();
    expect(behind).not.toHaveBeenCalled();
    act(() => outside.click());
    expect(behind).toHaveBeenCalledTimes(1);
    outside.remove();
  });

  it('does not open while disabled', () => {
    mountPicker({ disabled: true });
    expect(button().disabled).toBe(true);
  });
});

describe('workspace icon in the Edit dialog', () => {
  it('starts from the current icon and clears the draft on close', () => {
    wsCommands.openWorkspaceEditor('ws2');
    expect(wsSnap().draftIcon).toBe(ICON_A);
    wsCommands.closeWorkspaceEditor();
    expect(wsSnap().draftIcon).toBe('');
  });

  it('saves only the icon when only the icon changed, and closes', async () => {
    wsCommands.openWorkspaceEditor('ws2');
    wsCommands.setWorkspaceDraftIcon(ICON_B);
    expect(await wsCommands.saveWorkspaceEditor()).toBe(true);
    expect(calls).toEqual([['icon-ws', 'ws2', ICON_B]]);
    expect(wsSnap().editorOpen).toBe(false);
  });

  it('clearing the icon sends an empty id', async () => {
    wsCommands.openWorkspaceEditor('ws2');
    wsCommands.setWorkspaceDraftIcon('');
    await wsCommands.saveWorkspaceEditor();
    expect(calls).toEqual([['icon-ws', 'ws2', '']]);
  });

  it('calls nothing for an unchanged icon, and each changed field in order', async () => {
    wsCommands.openWorkspaceEditor('ws2');
    await wsCommands.saveWorkspaceEditor();
    expect(calls).toEqual([]);

    wsCommands.openWorkspaceEditor('ws2');
    wsCommands.setWorkspaceDraft('Wiki 2', 'directory');
    wsCommands.setWorkspaceDraftIcon(ICON_B);
    await wsCommands.saveWorkspaceEditor();
    expect(calls).toEqual([['rename-ws', 'ws2', 'Wiki 2'], ['view-ws', 'ws2', 'directory'], ['icon-ws', 'ws2', ICON_B]]);
  });

  it('stops at the first refusal and keeps the dialog open with the reason', async () => {
    wsCommands.openWorkspaceEditor('ws2');
    wsCommands.setWorkspaceDraft('Taken', 'timeline');
    wsCommands.setWorkspaceDraftIcon(ICON_B);
    failOn = 'rename-ws';
    failMessage = 'name already used';
    expect(await wsCommands.saveWorkspaceEditor()).toBe(false);
    expect(calls).toEqual([['rename-ws', 'ws2', 'Taken']]);
    expect(wsSnap()).toMatchObject({ editorOpen: true, error: 'name already used', draftIcon: ICON_B });

    calls.length = 0;
    wsCommands.openWorkspaceEditor('ws2');
    wsCommands.setWorkspaceDraftIcon(ICON_B);
    failOn = 'icon-ws';
    failMessage = 'Invalid icon';
    expect(await wsCommands.saveWorkspaceEditor()).toBe(false);
    expect(wsSnap()).toMatchObject({ editorOpen: true, error: 'Invalid icon' });
  });

  it('shows the picker left of the name, with Default for a workspace without an icon', () => {
    mount(NotesWorkspacesEditHost);
    act(() => host.querySelector('button.notes-cat-item[data-workspace="default"]').click());
    const row = host.querySelector('#notes-ws-edit-dialog .icon-name-row');
    expect(row.firstElementChild.classList.contains('icon-picker')).toBe(true);
    expect(row.querySelector('#notes-ws-edit-name')).not.toBeNull();
    expect(row.querySelector('[data-home-icon="notes"]')).not.toBeNull();
    act(() => wsCommands.closeWorkspaceEditor());
    act(() => host.querySelector('button.notes-cat-item[data-workspace="ws2"]').click());
    expect(host.querySelector(`#notes-ws-edit-dialog svg[data-icon-id="${ICON_A}"]`)).not.toBeNull();
  });

  it('Esc in the open grid leaves the dialog open, a second Esc closes the dialog', () => {
    mount(NotesWorkspacesEditHost);
    act(() => host.querySelector('button.notes-cat-item[data-workspace="ws2"]').click());
    act(() => host.querySelector('#notes-ws-edit-dialog .icon-picker-button').click());
    const esc = () =>
      act(() => {
        window.dispatchEvent(new KeyboardEvent('keydown', { key: 'Escape', bubbles: true, cancelable: true }));
      });
    esc();
    expect(document.body.querySelector('.icon-picker-panel')).toBeNull();
    expect(wsSnap().editorOpen).toBe(true);
    esc();
    expect(wsSnap().editorOpen).toBe(false);
  });
});

describe('workspace icon in the Add form', () => {
  it('submits the chosen icon with the create and clears it after', async () => {
    mount(NotesWorkspacesAddHost);
    expect(host.querySelector('.icon-name-row .icon-picker')).not.toBeNull();
    act(() => host.querySelector('.icon-picker-button').click());
    act(() => document.body.querySelector(`[data-icon-id="${ICON_B}"]`).click());
    const input = host.querySelector('#notes-workspace-new-name');
    act(() => {
      Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(input, 'New one');
      input.dispatchEvent(new Event('input', { bubbles: true }));
    });
    await act(async () => host.querySelector('#btn-notes-workspace-add').click());
    expect(calls).toEqual([['create-ws', 'New one', 'directory', ICON_B]]);
    expect(host.querySelector('.icon-picker-main svg[data-icon-id]')).toBeNull();
    expect(host.querySelector('.icon-picker-button [data-home-icon="notes"]')).not.toBeNull();
  });

  it('adds without an icon argument when none was chosen', async () => {
    await wsCommands.addWorkspace('Plain', 'timeline');
    expect(calls).toEqual([['create-ws', 'Plain', 'timeline']]);
  });
});

describe('project icon in the Edit dialog', () => {
  beforeEach(async () => {
    await catCommands.selectProjectsWorkspace('default');
  });

  it('reads the icon of each project and drafts it when the dialog opens', () => {
    expect(catSnap().rows.map((r) => r.icon)).toEqual(['', ICON_A]);
    catCommands.openNotesCategoryEditor('plans');
    expect(catSnap().draftIcon).toBe(ICON_A);
    catCommands.closeNotesCategoryEditor();
    expect(catSnap().draftIcon).toBe('');
  });

  it('sends the icon only when it changed', async () => {
    catCommands.openNotesCategoryEditor('plans');
    await catCommands.saveNotesCategory('default');
    expect(calls).toEqual([['update-p', 'default', 'plans', 'Plans', 'd']]);

    calls.length = 0;
    catCommands.openNotesCategoryEditor('plans');
    catCommands.setNotesCategoryDraftIcon(ICON_B);
    await catCommands.saveNotesCategory('default');
    expect(calls).toEqual([['update-p', 'default', 'plans', 'Plans', 'd', ICON_B]]);
    expect(catSnap().editorOpen).toBe(false);

    calls.length = 0;
    catCommands.openNotesCategoryEditor('plans');
    catCommands.setNotesCategoryDraftIcon('');
    await catCommands.saveNotesCategory('default');
    expect(calls).toEqual([['update-p', 'default', 'plans', 'Plans', 'd', '']]);
  });

  it('keeps the dialog open and shows the reason when the host refuses', async () => {
    catCommands.openNotesCategoryEditor('plans');
    catCommands.setNotesCategoryDraftIcon(ICON_B);
    failOn = 'update-p';
    failMessage = 'Invalid icon';
    await catCommands.saveNotesCategory('default');
    expect(catSnap()).toMatchObject({ editorOpen: true, error: 'Invalid icon', draftIcon: ICON_B });
  });

  it('shows the picker left of the title, defaulting to Folder', () => {
    mount(NotesCategoriesEditHost);
    act(() => host.querySelector('.notes-cat-row[data-project="inbox"] .notes-cat-row-open').click());
    const row = host.querySelector('#notes-cat-edit-dialog .icon-name-row');
    expect(row.firstElementChild.classList.contains('icon-picker')).toBe(true);
    expect(row.querySelector('#notes-cat-edit-title')).not.toBeNull();
    expect(row.querySelector(`svg[data-icon-id="${DEFAULT_PROJECT_ICON_ID}"]`)).not.toBeNull();
    expect(getIcon(DEFAULT_PROJECT_ICON_ID).name).toBe('Folder');
  });
});

describe('project icon in the Add form', () => {
  it('submits the chosen icon with the create', async () => {
    await catCommands.selectProjectsWorkspace('default');
    mount(NotesCategoriesAddHost);
    expect(host.querySelector('.icon-name-row .icon-picker')).not.toBeNull();
    act(() => host.querySelector('.icon-picker-button').click());
    act(() => document.body.querySelector(`[data-icon-id="${ICON_B}"]`).click());
    const input = host.querySelector('#notes-cat-new-title');
    act(() => {
      Object.getOwnPropertyDescriptor(HTMLInputElement.prototype, 'value').set.call(input, 'Ideas');
      input.dispatchEvent(new Event('input', { bubbles: true }));
    });
    await act(async () => host.querySelector('#btn-notes-cat-add').click());
    expect(calls).toEqual([['create-p', 'default', 'Ideas', '', ICON_B]]);
  });

  it('adds without an icon argument when none was chosen', async () => {
    await catCommands.addNotesCategory('default', 'Plain', '');
    expect(calls).toEqual([['create-p', 'default', 'Plain', '']]);
  });
});
