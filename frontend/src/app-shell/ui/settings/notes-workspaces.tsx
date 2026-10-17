import { useState, useSyncExternalStore } from 'react';
import type { ViewMode } from '../../../host/api.ts';
import { getStateVersion, state, subscribeState } from '../../../host/state.ts';
import {
  addWorkspace,
  changeWorkspaceViewMode,
  disableWorkspace,
  renameWorkspace,
} from '../../commands/settings/workspaces.ts';
import { workspacesPanelStore } from '../../state/settings/workspaces-panel.ts';
import { NotesWorkspacesDisabled } from './notes-workspaces-disabled.tsx';

type Draft = { id: string; name: string } | null;

function parseViewMode(value: string): ViewMode {
  return value === 'timeline' ? 'timeline' : 'directory';
}

function WorkspaceRow({ busy, workspaceId }: { busy: boolean; workspaceId: string }) {
  const workspace = state.workspaces.list.find((w) => w.id === workspaceId);
  const [draft, setDraft] = useState<Draft>(null);
  const [confirming, setConfirming] = useState(false);
  if (!workspace) return null;

  if (draft) {
    return (
      <div className="notes-cat-item">
        <input
          type="text"
          value={draft.name}
          autoComplete="off"
          spellCheck={false}
          disabled={busy}
          aria-label="Workspace name"
          onChange={(e) => setDraft({ id: draft.id, name: e.target.value })}
        />
        <button type="button" className="btn-settings-ghost" disabled={busy} onClick={() => setDraft(null)}>
          Cancel
        </button>
        <button
          type="button"
          className="btn-settings-save"
          disabled={busy || !draft.name.trim()}
          onClick={() => void renameWorkspace(draft.id, draft.name.trim()).then((ok) => ok && setDraft(null))}
        >
          Save
        </button>
      </div>
    );
  }

  return (
    <div className="notes-cat-item" data-workspace={workspace.id}>
      <span className="notes-cat-item-main">
        <span className="notes-cat-item-title-row">
          <span className="notes-cat-item-title">{workspace.name}</span>
          {workspace.is_default ? <span className="notes-cat-badge">Built-in</span> : null}
        </span>
      </span>
      <select
        className="notes-ws-view"
        aria-label={`View of ${workspace.name}`}
        value={workspace.view_mode}
        disabled={busy}
        onChange={(e) => void changeWorkspaceViewMode(workspace.id, parseViewMode(e.target.value))}
      >
        <option value="directory">Directory</option>
        <option value="timeline">Timeline</option>
      </select>
      <button
        type="button"
        className="btn-settings-ghost"
        disabled={busy}
        onClick={() => setDraft({ id: workspace.id, name: workspace.name })}
      >
        Rename
      </button>
      {workspace.is_default ? null : confirming ? (
        <>
          <button type="button" className="btn-settings-ghost" disabled={busy} onClick={() => setConfirming(false)}>
            Cancel
          </button>
          <button
            type="button"
            className="sediment-kb-delete-btn"
            disabled={busy}
            onClick={() => void disableWorkspace(workspace.id).then((ok) => ok && setConfirming(false))}
          >
            Confirm disable
          </button>
        </>
      ) : (
        <button
          type="button"
          className="sediment-kb-delete-btn"
          disabled={busy}
          onClick={() => setConfirming(true)}
        >
          Disable
        </button>
      )}
    </div>
  );
}

export function NotesWorkspacesHost() {
  useSyncExternalStore(subscribeState, getStateVersion, getStateVersion);
  const snap = useSyncExternalStore(workspacesPanelStore.subscribe, workspacesPanelStore.getSnapshot);
  const [name, setName] = useState('');
  const [viewMode, setViewMode] = useState<ViewMode>('directory');
  const { busy } = snap;

  async function onAdd() {
    const trimmed = name.trim();
    if (!trimmed) return;
    if (await addWorkspace(trimmed, viewMode)) setName('');
  }

  return (
    <div className="notes-cat-add">
      <p className="notes-cat-lead">
        A workspace holds its own projects and documents. Disabling a workspace hides it; nothing is deleted,
        and it can be enabled again from the Disabled list. The built-in workspace can be renamed but not disabled.
      </p>
      <div id="notes-workspace-list">
        {state.workspaces.list.map((workspace) => (
          <WorkspaceRow key={workspace.id} workspaceId={workspace.id} busy={busy} />
        ))}
      </div>
      <NotesWorkspacesDisabled busy={busy} rows={snap.disabled} />
      <div className="settings-field">
        <label htmlFor="notes-workspace-new-name">New workspace</label>
        <input
          id="notes-workspace-new-name"
          type="text"
          value={name}
          placeholder="Name"
          autoComplete="off"
          spellCheck={false}
          disabled={busy}
          onChange={(e) => setName(e.target.value)}
        />
      </div>
      <div className="settings-field">
        <label htmlFor="notes-workspace-new-view">View</label>
        <select
          id="notes-workspace-new-view"
          value={viewMode}
          disabled={busy}
          onChange={(e) => setViewMode(parseViewMode(e.target.value))}
        >
          <option value="directory">Directory</option>
          <option value="timeline">Timeline</option>
        </select>
      </div>
      <div id="notes-workspace-error" className="settings-result notes-cat-error">
        {snap.error}
      </div>
      <div className="settings-panel-actions">
        <button
          id="btn-notes-workspace-add"
          type="button"
          className="btn-settings-save"
          disabled={busy || !name.trim()}
          onClick={() => void onAdd()}
        >
          {busy ? 'Working…' : 'Add workspace'}
        </button>
      </div>
    </div>
  );
}
