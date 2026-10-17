import { enableWorkspace } from '../../commands/settings/workspaces.ts';
import type { DisabledWorkspaceRow } from '../../state/settings/workspaces-panel.ts';

/** Collapsed list of disabled workspaces; each can be enabled again. */
export function NotesWorkspacesDisabled({
  busy,
  rows,
}: {
  busy: boolean;
  rows: DisabledWorkspaceRow[];
}) {
  if (!rows.length) return null;
  return (
    <details id="notes-workspaces-disabled" className="notes-disabled">
      <summary>Disabled ({rows.length})</summary>
      <p className="notes-cat-lead">
        Disabled workspaces are hidden everywhere. Their documents are kept, and the name stays reserved.
      </p>
      {rows.map((row) => (
        <div key={row.id} className="notes-cat-item" data-workspace-disabled={row.id}>
          <span className="notes-cat-item-main">
            <span className="notes-cat-item-title">{row.name}</span>
          </span>
          <button
            type="button"
            className="btn-settings-ghost"
            disabled={busy}
            onClick={() => void enableWorkspace(row.id)}
          >
            Enable
          </button>
        </div>
      ))}
    </details>
  );
}
