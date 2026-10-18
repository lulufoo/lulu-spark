import { useSyncExternalStore } from 'react';
import { getStateVersion, state, subscribeState } from '../../../host/state.ts';
import { notesCategoriesStore } from '../../state/settings/notes-categories.ts';
import { workspacesPanelStore } from '../../state/settings/workspaces-panel.ts';
import { HiddenFilesTab } from './hidden-files-tab.tsx';
import { NotesCategoriesAddHost } from './notes-categories-add.tsx';
import { NotesCategoriesEditHost } from './notes-categories-edit.tsx';

/**
 * One page per workspace nav item. The tabs always render (their click listeners are wired once on
 * the static DOM); the contents follow the selected workspace.
 */
export function WorkspaceProjectsPanel() {
  useSyncExternalStore(subscribeState, getStateVersion, getStateVersion);
  const snap = useSyncExternalStore(notesCategoriesStore.subscribe, notesCategoriesStore.getSnapshot);
  const { root } = useSyncExternalStore(workspacesPanelStore.subscribe, workspacesPanelStore.getSnapshot);
  const workspace = state.workspaces.list.find((w) => w.workspace_id === snap.workspaceId);
  const dir = workspace && root ? `${root}/${workspace.workspace_id}` : '';

  return (
    <div id="settings-panel-workspace" className="settings-panel">
      <div className="settings-tabs" role="tablist">
        <button type="button" className="settings-tab active" data-tab="directory" role="tab">
          Directory
        </button>
        <button type="button" className="settings-tab" data-tab="add" role="tab">
          Add project
        </button>
        <button type="button" className="settings-tab" data-tab="edit" role="tab">
          Edit project
        </button>
        <button type="button" className="settings-tab" data-tab="hidden" role="tab">
          Hidden files
        </button>
      </div>
      <div
        id="settings-tab-workspace-directory"
        className="settings-tab-panel active"
        data-tab="directory"
      >
        <div className="settings-field">
          <label htmlFor="workspace-dir-path">Workspace directory</label>
          <input
            id="workspace-dir-path"
            type="text"
            spellCheck={false}
            autoComplete="off"
            readOnly
            className="settings-input-readonly"
            value={dir}
            placeholder={dir || undefined}
          />
          <span className="settings-field-hint">
            {workspace
              ? `Folder of ${workspace.workspace_name}. Its projects live inside.`
              : 'Choose a workspace.'}
          </span>
        </div>
      </div>
      <div id="settings-tab-workspace-add" className="settings-tab-panel" data-tab="add">
        <NotesCategoriesAddHost />
      </div>
      <div id="settings-tab-workspace-edit" className="settings-tab-panel" data-tab="edit">
        <NotesCategoriesEditHost />
      </div>
      <HiddenFilesTab />
    </div>
  );
}
