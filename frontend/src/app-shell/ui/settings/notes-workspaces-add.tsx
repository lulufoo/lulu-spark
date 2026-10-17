import { useState, useSyncExternalStore } from 'react';
import type { ViewMode } from '../../../host/api.ts';
import { addWorkspace } from '../../commands/settings/workspaces.ts';
import { defaultWorkspaceIconName, HomeNavIcon } from '../../../home/ui/nav-icons.tsx';
import { Select } from '../../../shared/components/select/index.ts';
import { IconPicker } from './icon-picker.tsx';
import { workspacesPanelStore } from '../../state/settings/workspaces-panel.ts';

/** The two ways a workspace can show its documents. */
export const VIEW_MODE_OPTIONS = [
  { value: 'directory', label: 'Directory' },
  { value: 'timeline', label: 'Timeline' },
];

export function parseViewMode(value: string): ViewMode {
  return value === 'timeline' ? 'timeline' : 'directory';
}

/** Add form; it clears only after the host accepted the workspace. */
export function NotesWorkspacesAddHost() {
  const snap = useSyncExternalStore(workspacesPanelStore.subscribe, workspacesPanelStore.getSnapshot);
  const [name, setName] = useState('');
  const [viewMode, setViewMode] = useState<ViewMode>('directory');
  const [icon, setIcon] = useState('');
  const { busy } = snap;

  async function onAdd() {
    const trimmed = name.trim();
    if (!trimmed) return;
    if (await addWorkspace(trimmed, viewMode, icon)) {
      setName('');
      setIcon('');
    }
  }

  return (
    <div className="notes-cat-add">
      <p className="notes-cat-lead">
        A workspace holds its own projects and documents. Name and view can be changed later.
      </p>
      <div className="settings-field">
        <label htmlFor="notes-workspace-new-name">Name</label>
        <div className="icon-name-row">
          <IconPicker
            label="Workspace icon"
            value={icon}
            defaultIcon={<HomeNavIcon name={defaultWorkspaceIconName('')} />}
            disabled={busy}
            onChange={setIcon}
          />
          <input
            id="notes-workspace-new-name"
            type="text"
            value={name}
            placeholder="Display name"
            autoComplete="off"
            spellCheck={false}
            disabled={busy}
            onChange={(e) => setName(e.target.value)}
          />
        </div>
      </div>
      <div className="settings-field">
        <label>View</label>
        <Select
          ariaLabel="View"
          value={viewMode}
          options={VIEW_MODE_OPTIONS}
          disabled={busy}
          onChange={(next) => setViewMode(parseViewMode(next))}
        />
      </div>
      <div id="notes-workspace-add-error" className="settings-result notes-cat-error">
        {snap.editorOpen ? '' : snap.error}
      </div>
      <div className="settings-panel-actions">
        <button
          id="btn-notes-workspace-add"
          type="button"
          className="btn-settings-save"
          disabled={busy || !name.trim()}
          onClick={() => void onAdd()}
        >
          {busy ? 'Adding…' : 'Add workspace'}
        </button>
      </div>
    </div>
  );
}
