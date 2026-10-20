import { createModuleStore } from '../../../shared/module-store.ts';
import type { ViewMode } from '../../../host/api.ts';

export type DisabledWorkspaceRow = { id: string; name: string; sortIndex: number };

export type WorkspacesPanelSnap = {
  /** Fixed workspaces root on disk; shown read-only in the Directory tab. */
  root: string;
  /** Workspaces the person disabled; they can be enabled again. */
  disabled: DisabledWorkspaceRow[];
  error: string;
  busy: boolean;
  /** Workspace open in the edit dialog; empty when none. */
  selectedId: string;
  editorOpen: boolean;
  draftName: string;
  draftViewMode: ViewMode;
  /** Icon chosen in the dialog; empty = the default icon. */
  draftIcon: string;
  /** Description chosen in the dialog; empty = none. */
  draftDescription: string;
  /** The edit dialog asks "Disable <name>?" instead of showing the form. */
  confirmDisable: boolean;
};

export function emptyWorkspacesPanelSnap(): WorkspacesPanelSnap {
  return {
    root: '',
    disabled: [],
    error: '',
    busy: false,
    selectedId: '',
    editorOpen: false,
    draftName: '',
    draftViewMode: 'directory',
    draftIcon: '',
    draftDescription: '',
    confirmDisable: false,
  };
}

export const workspacesPanelStore = createModuleStore<WorkspacesPanelSnap>(emptyWorkspacesPanelSnap());

export function patchWorkspacesPanel(partial: Partial<WorkspacesPanelSnap>) {
  workspacesPanelStore.set({ ...workspacesPanelStore.getSnapshot(), ...partial });
}
