import { useState, useSyncExternalStore } from 'react';
import * as api from '../../../host/api.ts';
import { getStateVersion, state, subscribeState } from '../../../host/state.ts';
import { loadWorkspaces } from '../../../notes/commands/workspace.ts';

type Draft = { id: string; name: string } | null;

function describeError(err: unknown): string {
  return err instanceof Error ? err.message : String(err);
}

function throwOnError(data: unknown) {
  const error = (data as { error?: string } | null)?.error;
  if (error) throw new Error(error);
}

/** Reload the registry; the page reloads its index when the shown workspace changed. */
async function refreshWorkspaces(previous: string) {
  await loadWorkspaces();
  if (state.workspaces.current !== previous) document.dispatchEvent(new CustomEvent('cta:reload'));
}

function WorkspaceRow({
  busy,
  onRename,
  onDelete,
  workspaceId,
}: {
  busy: boolean;
  onRename: (id: string, name: string) => Promise<boolean>;
  onDelete: (id: string) => Promise<boolean>;
  workspaceId: string;
}) {
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
          onClick={() => void onRename(draft.id, draft.name.trim()).then((ok) => ok && setDraft(null))}
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
        <span className="notes-cat-item-desc">{workspace.view_mode}</span>
      </span>
      <button
        type="button"
        className="btn-settings-ghost"
        disabled={busy}
        onClick={() => setDraft({ id: workspace.id, name: workspace.name })}
      >
        Rename
      </button>
      {workspace.is_default ? null : confirming ? (
        <button
          type="button"
          className="sediment-kb-delete-btn"
          disabled={busy}
          onClick={() => void onDelete(workspace.id).then((ok) => ok && setConfirming(false))}
        >
          Confirm delete
        </button>
      ) : (
        <button
          type="button"
          className="sediment-kb-delete-btn"
          disabled={busy}
          onClick={() => setConfirming(true)}
        >
          Delete
        </button>
      )}
    </div>
  );
}

export function NotesWorkspacesHost() {
  useSyncExternalStore(subscribeState, getStateVersion, getStateVersion);
  const [name, setName] = useState('');
  const [viewMode, setViewMode] = useState<api.ViewMode>('directory');
  const [error, setError] = useState('');
  const [busy, setBusy] = useState(false);

  async function run(call: () => Promise<unknown>): Promise<boolean> {
    const previous = state.workspaces.current;
    setError('');
    setBusy(true);
    try {
      throwOnError(await call());
      await refreshWorkspaces(previous);
      return true;
    } catch (err) {
      setError(describeError(err));
      return false;
    } finally {
      setBusy(false);
    }
  }

  async function addWorkspace() {
    const trimmed = name.trim();
    if (!trimmed) return;
    if (await run(() => api.createWorkspace(trimmed, viewMode))) setName('');
  }

  return (
    <div className="notes-cat-add">
      <p className="notes-cat-lead">
        A workspace holds its own projects and documents. The built-in workspace can be renamed but not deleted;
        a workspace that still holds documents cannot be deleted.
      </p>
      <div id="notes-workspace-list">
        {state.workspaces.list.map((workspace) => (
          <WorkspaceRow
            key={workspace.id}
            workspaceId={workspace.id}
            busy={busy}
            onRename={(id, next) => run(() => api.renameWorkspace(id, next))}
            onDelete={(id) => run(() => api.disableWorkspace(id))}
          />
        ))}
      </div>
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
          onChange={(e) => setViewMode(e.target.value === 'timeline' ? 'timeline' : 'directory')}
        >
          <option value="directory">Directory</option>
          <option value="timeline">Timeline</option>
        </select>
      </div>
      <div id="notes-workspace-error" className="settings-result notes-cat-error">
        {error}
      </div>
      <div className="settings-panel-actions">
        <button
          id="btn-notes-workspace-add"
          type="button"
          className="btn-settings-save"
          disabled={busy || !name.trim()}
          onClick={() => void addWorkspace()}
        >
          {busy ? 'Working…' : 'Add workspace'}
        </button>
      </div>
    </div>
  );
}
