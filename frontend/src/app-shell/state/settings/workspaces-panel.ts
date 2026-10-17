import { createModuleStore } from '../../../shared/module-store.ts';

export type DisabledWorkspaceRow = { id: string; name: string };

export type WorkspacesPanelSnap = {
  /** Workspaces the person disabled; they can be enabled again. */
  disabled: DisabledWorkspaceRow[];
  error: string;
  busy: boolean;
};

export function emptyWorkspacesPanelSnap(): WorkspacesPanelSnap {
  return { disabled: [], error: '', busy: false };
}

export const workspacesPanelStore = createModuleStore<WorkspacesPanelSnap>(emptyWorkspacesPanelSnap());

export function patchWorkspacesPanel(partial: Partial<WorkspacesPanelSnap>) {
  workspacesPanelStore.set({ ...workspacesPanelStore.getSnapshot(), ...partial });
}
