import * as api from '../../../host/api.ts';
import { state } from '../../../host/state.ts';
import { announceRegistryChange } from '../../../shared/registry-events.ts';
import { loadWorkspaces } from '../../../notes/commands/workspace.ts';
import { applyViewModeChange } from '../../../notes/commands/workspace-switch.ts';
import {
  patchWorkspacesPanel,
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
  const list = (data as { workspaces?: Array<{ id?: unknown; name?: unknown }> } | null)?.workspaces ?? [];
  const rows: DisabledWorkspaceRow[] = [];
  for (const item of list) {
    if (typeof item.id !== 'string' || !item.id) continue;
    rows.push({ id: item.id, name: typeof item.name === 'string' && item.name ? item.name : item.id });
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

export function addWorkspace(name: string, viewMode: api.ViewMode) {
  return run(() => api.createWorkspace(name, viewMode));
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

export function disableWorkspace(id: string) {
  return run(() => api.disableWorkspace(id));
}

export function enableWorkspace(id: string) {
  return run(() => api.enableWorkspace(id));
}
