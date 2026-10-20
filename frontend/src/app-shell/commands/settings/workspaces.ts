import * as api from '../../../host/api.ts';
import { notifyState, state } from '../../../host/state.ts';
import { announceRegistryChange } from '../../../shared/registry-events.ts';
import { withDisabledInPlace } from '../../../shared/reorder.ts';
import { loadWorkspaces } from '../../../notes/commands/workspace.ts';
import { applyViewModeChange } from '../../../notes/commands/workspace-switch.ts';
import {
  patchWorkspacesPanel,
  workspacesPanelStore,
  type DisabledWorkspaceRow,
} from '../../state/settings/workspaces-panel.ts';

function describeError(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

function throwOnError(data: unknown) {
  const error = (data as { error?: string } | null)?.error;
  if (error) throw new Error(error);
}

function asDisabledRows(data: unknown): DisabledWorkspaceRow[] {
  throwOnError(data);
  const list =
    (data as { workspaces?: Array<{ workspace_id?: unknown; workspace_name?: unknown; sort_index?: unknown }> } | null)?.workspaces ?? [];
  const rows: DisabledWorkspaceRow[] = [];
  for (const [at, item] of list.entries()) {
    if (typeof item.workspace_id !== 'string' || !item.workspace_id) continue;
    rows.push({
      id: item.workspace_id,
      name: typeof item.workspace_name === 'string' && item.workspace_name ? item.workspace_name : item.workspace_id,
      sortIndex: typeof item.sort_index === 'number' ? item.sort_index : at,
    });
  }
  return rows;
}

/** Fill the "Disabled" section of the Workspaces panel. */
export async function loadDisabledWorkspaces() {
  try {
    patchWorkspacesPanel({ disabled: asDisabledRows(await api.fetchDisabledWorkspaces()) });
  } catch (err) {
    patchWorkspacesPanel({ error: describeError(err) });
  }
}

/**
 * Run one registry change, then reload the registry and the disabled list. The page reloads
 * its index when the workspace it showed is gone (for example it was just disabled).
 */
async function run(call: () => Promise<unknown>, after?: () => void): Promise<boolean> {
  const previous = state.workspaces.current;
  patchWorkspacesPanel({ error: '', busy: true });
  try {
    throwOnError(await call());
    await loadWorkspaces();
    if (state.workspaces.current !== previous) document.dispatchEvent(new CustomEvent('cta:reload'));
    after?.();
    announceRegistryChange();
    patchWorkspacesPanel({ disabled: asDisabledRows(await api.fetchDisabledWorkspaces()) });
    return true;
  } catch (err) {
    patchWorkspacesPanel({ error: describeError(err) });
    return false;
  } finally {
    patchWorkspacesPanel({ busy: false });
  }
}

export function clearWorkspacesError() {
  patchWorkspacesPanel({ error: '' });
}

/** `icon` empty = the default icon. `description` empty is omitted from the create. */
export function addWorkspace(name: string, viewMode: api.ViewMode, icon = '', description = '') {
  const note = description.trim();
  return run(() => {
    if (icon) return note ? api.createWorkspace(name, viewMode, icon, note) : api.createWorkspace(name, viewMode, icon);
    return note ? api.createWorkspace(name, viewMode, undefined, note) : api.createWorkspace(name, viewMode);
  });
}

export function renameWorkspace(id: string, name: string) {
  return run(() => api.renameWorkspace(id, name));
}

/** Directory or timeline is a property of the workspace; the page regroups when it is the shown one. */
export function changeWorkspaceViewMode(id: string, mode: api.ViewMode) {
  return run(
    () => api.setWorkspaceViewMode(id, mode),
    () => {
      if (id === state.workspaces.current) applyViewModeChange();
    },
  );
}

/** Empty `icon` clears it, back to the default icon. */
export function changeWorkspaceIcon(id: string, icon: string) {
  return run(() => api.setWorkspaceIcon(id, icon));
}

/** Empty `description` clears it. */
export function changeWorkspaceDescription(id: string, description: string) {
  return run(() => api.setWorkspaceDescription(id, description));
}

/**
 * Drag-reorder: show the new order at once, then ask the backend; a refusal puts the old order
 * back and says why. `ids` are the enabled workspaces in their new order, disabled ones keep
 * their place.
 */
export async function reorderWorkspaceList(ids: string[]): Promise<boolean> {
  const previous = state.workspaces.list;
  const byId = new Map(previous.map((w) => [w.workspace_id, w]));
  const next = ids.map((id) => byId.get(id)).filter((w): w is NonNullable<typeof w> => Boolean(w));
  if (next.length !== previous.length) return false;
  const ranked = previous.map((w, at) => ({ id: w.workspace_id, sortIndex: w.sort_index ?? at }));
  const disabled = workspacesPanelStore.getSnapshot().disabled;
  const whole = withDisabledInPlace(ranked, disabled, ids);
  state.workspaces.list = next;
  notifyState();
  patchWorkspacesPanel({ error: '' });
  try {
    throwOnError(await api.reorderWorkspaces(whole));
    await loadWorkspaces();
    announceRegistryChange();
    patchWorkspacesPanel({ disabled: asDisabledRows(await api.fetchDisabledWorkspaces()) });
    return true;
  } catch (err) {
    state.workspaces.list = previous;
    notifyState();
    patchWorkspacesPanel({ error: `Could not change the order: ${describeError(err)}` });
    return false;
  }
}

export function disableWorkspace(id: string) {
  return run(() => api.disableWorkspace(id));
}

export function enableWorkspace(id: string) {
  return run(() => api.enableWorkspace(id));
}

/** Open the edit dialog for one workspace, with its current name and view in the draft. */
export function openWorkspaceEditor(id: string) {
  const workspace = state.workspaces.list.find((w) => w.workspace_id === id);
  if (!workspace) return;
  patchWorkspacesPanel({
    selectedId: workspace.workspace_id,
    editorOpen: true,
    draftName: workspace.workspace_name,
    draftViewMode: workspace.view_mode,
    draftIcon: workspace.icon,
    draftDescription: workspace.description ?? '',
    confirmDisable: false,
    error: '',
  });
}

export function closeWorkspaceEditor() {
  const snap = workspacesPanelStore.getSnapshot();
  if (!snap.editorOpen && !snap.selectedId) return;
  patchWorkspacesPanel({
    selectedId: '',
    editorOpen: false,
    draftName: '',
    draftViewMode: 'directory',
    draftIcon: '',
    draftDescription: '',
    confirmDisable: false,
    error: '',
  });
}

export function setWorkspaceDraft(name: string, viewMode: api.ViewMode) {
  patchWorkspacesPanel({ draftName: name, draftViewMode: viewMode });
}

export function setWorkspaceDraftIcon(icon: string) {
  patchWorkspacesPanel({ draftIcon: icon });
}

export function setWorkspaceDraftDescription(description: string) {
  patchWorkspacesPanel({ draftDescription: description });
}

/** The built-in workspace cannot be disabled. */
export function askWorkspaceDisable() {
  const { selectedId } = workspacesPanelStore.getSnapshot();
  if (!selectedId || state.workspaces.list.find((w) => w.workspace_id === selectedId)?.is_default) return;
  patchWorkspacesPanel({ confirmDisable: true, error: '' });
}

export function cancelWorkspaceDisable() {
  patchWorkspacesPanel({ confirmDisable: false, error: '' });
}

/**
 * Save the dialog: rename, change the view, icon and description, each only when it differs, then
 * close it. Stops at the first refusal and leaves the dialog open with the reason.
 */
export async function saveWorkspaceEditor(): Promise<boolean> {
  const snap = workspacesPanelStore.getSnapshot();
  const workspace = state.workspaces.list.find((w) => w.workspace_id === snap.selectedId);
  const name = snap.draftName.trim();
  if (!workspace || !name) return false;
  const viewChanged = snap.draftViewMode !== workspace.view_mode;
  const iconChanged = snap.draftIcon !== workspace.icon;
  const nextDescription = snap.draftDescription.trim();
  const descriptionChanged = nextDescription !== (workspace.description ?? '');
  if (name !== workspace.workspace_name && !(await renameWorkspace(workspace.workspace_id, name))) return false;
  if (viewChanged && !(await changeWorkspaceViewMode(workspace.workspace_id, snap.draftViewMode))) return false;
  if (iconChanged && !(await changeWorkspaceIcon(workspace.workspace_id, snap.draftIcon))) return false;
  if (descriptionChanged && !(await changeWorkspaceDescription(workspace.workspace_id, nextDescription))) return false;
  closeWorkspaceEditor();
  return true;
}

export async function confirmWorkspaceDisable(): Promise<boolean> {
  const { selectedId } = workspacesPanelStore.getSnapshot();
  if (!selectedId || !(await disableWorkspace(selectedId))) return false;
  closeWorkspaceEditor();
  return true;
}
