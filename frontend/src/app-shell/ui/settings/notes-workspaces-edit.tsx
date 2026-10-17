import { useEffect, useSyncExternalStore } from 'react';
import { getStateVersion, state, subscribeState } from '../../../host/state.ts';
import { ReorderRows } from '../../../shared/reorder-rows.tsx';
import { OverlayDismissButton } from '../../../shared/overlay-dismiss-button.tsx';
import {
  askWorkspaceDisable,
  cancelWorkspaceDisable,
  closeWorkspaceEditor,
  confirmWorkspaceDisable,
  enableWorkspace,
  openWorkspaceEditor,
  reorderWorkspaceList,
  saveWorkspaceEditor,
  setWorkspaceDraft,
  setWorkspaceDraftIcon,
} from '../../commands/settings/workspaces.ts';
import { workspacesPanelStore } from '../../state/settings/workspaces-panel.ts';
import { defaultWorkspaceIconName, HomeNavIcon } from '../../../home/ui/nav-icons.tsx';
import { IconPicker } from './icon-picker.tsx';
import { parseViewMode } from './notes-workspaces-add.tsx';

/** Workspace list (whole row opens the dialog), the edit dialog, and the collapsed Disabled list. */
export function NotesWorkspacesEditHost() {
  useSyncExternalStore(subscribeState, getStateVersion, getStateVersion);
  const snap = useSyncExternalStore(workspacesPanelStore.subscribe, workspacesPanelStore.getSnapshot);
  const selected = state.workspaces.list.find((w) => w.id === snap.selectedId);
  const builtIn = Boolean(selected?.is_default);

  useEffect(() => {
    if (!snap.editorOpen) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key !== 'Escape' || snap.busy) return;
      e.preventDefault();
      if (snap.confirmDisable) cancelWorkspaceDisable();
      else closeWorkspaceEditor();
    };
    window.addEventListener('keydown', onKey);
    return () => window.removeEventListener('keydown', onKey);
  }, [snap.editorOpen, snap.confirmDisable, snap.busy]);

  useEffect(() => {
    if (!snap.editorOpen || snap.confirmDisable) return;
    queueMicrotask(() => document.getElementById('notes-ws-edit-name')?.focus());
  }, [snap.editorOpen, snap.confirmDisable, snap.selectedId]);

  return (
    <>
      <p className="notes-cat-lead">
        Click a workspace to rename it or change its view; drag the handle to change the order. Disabling a workspace hides it; nothing is
        deleted.
      </p>
      {snap.error && !snap.editorOpen ? (
        <div id="notes-workspace-list-error" className="settings-result notes-cat-error">
          {snap.error}
        </div>
      ) : null}
      <div id="notes-ws-list">
        <ReorderRows
          disabled={snap.busy}
          onReorder={(ids) => void reorderWorkspaceList(ids)}
          rows={state.workspaces.list.map((workspace) => ({
            id: workspace.id,
            content: (
              <button
                type="button"
                className="notes-cat-item"
                data-workspace={workspace.id}
                disabled={snap.busy}
                onClick={() => openWorkspaceEditor(workspace.id)}
              >
                <span className="notes-cat-item-main">
                  <span className="notes-cat-item-title-row">
                    <span className="notes-cat-item-title">{workspace.name}</span>
                    {workspace.is_default ? <span className="notes-cat-badge">Built-in</span> : null}
                  </span>
                  <span className="notes-cat-item-desc">
                    {workspace.view_mode === 'timeline' ? 'Timeline view' : 'Directory view'}
                  </span>
                </span>
                <span className="notes-cat-item-action">Edit</span>
              </button>
            ),
          }))}
        />
        {snap.disabled.length ? (
          <details id="notes-workspaces-disabled" className="notes-disabled">
            <summary className="notes-cat-lead">Disabled ({snap.disabled.length})</summary>
            {snap.disabled.map((row) => (
              <button
                key={row.id}
                type="button"
                className="notes-cat-item"
                data-workspace-disabled={row.id}
                disabled={snap.busy}
                onClick={() => void enableWorkspace(row.id)}
              >
                <span className="notes-cat-item-main">
                  <span className="notes-cat-item-title">{row.name}</span>
                </span>
                <span className="notes-cat-item-action">Enable</span>
              </button>
            ))}
          </details>
        ) : null}
      </div>
      <div
        id="notes-ws-edit-dialog"
        className={snap.editorOpen ? 'open' : undefined}
        role="presentation"
        onClick={(e) => {
          if (e.target === e.currentTarget && !snap.busy) closeWorkspaceEditor();
        }}
      >
        {selected ? (
          <div
            id="notes-ws-edit-dialog-box"
            role="dialog"
            aria-modal="true"
            aria-labelledby="notes-ws-edit-dialog-title"
          >
            <div id="notes-ws-edit-dialog-header">
              <h3 id="notes-ws-edit-dialog-title">
                {snap.confirmDisable ? 'Disable workspace' : 'Edit workspace'}
              </h3>
              <OverlayDismissButton
                id="btn-notes-ws-edit-close"
                disabled={snap.busy}
                title="Close"
                onClick={() => closeWorkspaceEditor()}
              />
            </div>
            {snap.confirmDisable ? (
              <div id="notes-ws-edit-dialog-body">
                <p className="notes-cat-confirm-copy">
                  Disable <strong>{selected.name}</strong>? It is hidden everywhere. Its documents are
                  kept, the name stays reserved, and you can enable it again from the Disabled list.
                </p>
                <div id="notes-ws-edit-error" className="settings-result notes-cat-error">
                  {snap.error}
                </div>
                <div className="notes-cat-edit-actions">
                  <button
                    type="button"
                    className="btn-settings-ghost"
                    disabled={snap.busy}
                    onClick={() => cancelWorkspaceDisable()}
                  >
                    Back
                  </button>
                  <button
                    id="btn-notes-ws-disable"
                    type="button"
                    className="sediment-kb-delete-btn"
                    disabled={snap.busy}
                    onClick={() => void confirmWorkspaceDisable()}
                  >
                    {snap.busy ? 'Disabling…' : 'Disable workspace'}
                  </button>
                </div>
              </div>
            ) : (
              <div id="notes-ws-edit-dialog-body">
                <div className="settings-field">
                  <label htmlFor="notes-ws-edit-name">Name</label>
                  <div className="icon-name-row">
                    <IconPicker
                      label="Workspace icon"
                      value={snap.draftIcon}
                      defaultIcon={<HomeNavIcon name={defaultWorkspaceIconName(selected.id)} />}
                      disabled={snap.busy}
                      onChange={setWorkspaceDraftIcon}
                    />
                    <input
                      id="notes-ws-edit-name"
                      type="text"
                      value={snap.draftName}
                      autoComplete="off"
                      spellCheck={false}
                      disabled={snap.busy}
                      onChange={(e) => setWorkspaceDraft(e.target.value, snap.draftViewMode)}
                      onKeyDown={(e) => {
                        if (e.key === 'Enter') {
                          e.preventDefault();
                          void saveWorkspaceEditor();
                        }
                      }}
                    />
                  </div>
                </div>
                <div className="settings-field">
                  <label htmlFor="notes-ws-edit-view">View</label>
                  <select
                    id="notes-ws-edit-view"
                    value={snap.draftViewMode}
                    disabled={snap.busy}
                    onChange={(e) => setWorkspaceDraft(snap.draftName, parseViewMode(e.target.value))}
                  >
                    <option value="directory">Directory</option>
                    <option value="timeline">Timeline</option>
                  </select>
                </div>
                <div id="notes-ws-edit-error" className="settings-result notes-cat-error">
                  {snap.error}
                </div>
                <div className="notes-cat-edit-actions">
                  {builtIn ? (
                    <span className="notes-cat-edit-actions-spacer" />
                  ) : (
                    <button
                      type="button"
                      className="sediment-kb-delete-btn"
                      disabled={snap.busy}
                      onClick={() => askWorkspaceDisable()}
                    >
                      Disable
                    </button>
                  )}
                  <button
                    type="button"
                    className="btn-settings-ghost"
                    disabled={snap.busy}
                    onClick={() => closeWorkspaceEditor()}
                  >
                    Cancel
                  </button>
                  <button
                    id="btn-notes-ws-save"
                    type="button"
                    className="btn-settings-save"
                    disabled={snap.busy || !snap.draftName.trim()}
                    onClick={() => void saveWorkspaceEditor()}
                  >
                    {snap.busy ? 'Saving…' : 'Save'}
                  </button>
                </div>
              </div>
            )}
          </div>
        ) : null}
      </div>
    </>
  );
}
