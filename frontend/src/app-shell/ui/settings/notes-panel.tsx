import { useSyncExternalStore } from 'react';
import { workspacesPanelStore } from '../../state/settings/workspaces-panel.ts';
import { NotesWorkspacesAddHost } from './notes-workspaces-add.tsx';
import { NotesWorkspacesEditHost } from './notes-workspaces-edit.tsx';

/** The Workspaces panel manages workspaces only; projects live on each workspace's own page. */
export function NotesSettingsPanel() {
  const snap = useSyncExternalStore(workspacesPanelStore.subscribe, workspacesPanelStore.getSnapshot);
  return (
    <div id="settings-panel-notes" className="settings-panel active">
      <div className="settings-tabs" role="tablist">
        <button type="button" className="settings-tab active" data-tab="directory" role="tab">
          Directory
        </button>
        <button type="button" className="settings-tab" data-tab="add" role="tab">
          Add workspace
        </button>
        <button type="button" className="settings-tab" data-tab="edit" role="tab">
          Edit &amp; Disable workspace
        </button>
      </div>
      <div
        id="settings-tab-notes-directory"
        className="settings-tab-panel active"
        data-tab="directory"
      >
        <div className="settings-field">
          <label htmlFor="workspaces-root-path">Workspaces directory</label>
          <input
            id="workspaces-root-path"
            type="text"
            spellCheck={false}
            autoComplete="off"
            readOnly
            className="settings-input-readonly"
            value={snap.root}
            placeholder={snap.root || undefined}
          />
          <span className="settings-field-hint">
            Fixed workspaces root. Not configurable.
          </span>
        </div>
      </div>
      <div id="settings-tab-notes-add" className="settings-tab-panel" data-tab="add">
        <NotesWorkspacesAddHost />
      </div>
      <div id="settings-tab-notes-edit" className="settings-tab-panel" data-tab="edit">
        <NotesWorkspacesEditHost />
      </div>
    </div>
  );
}
