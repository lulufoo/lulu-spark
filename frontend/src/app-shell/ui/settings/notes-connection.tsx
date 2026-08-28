import { useSyncExternalStore } from 'react';
import { deleteNotesGithubRepo } from '../../commands/settings/notes-github.ts';
import { notesConnectionStore } from '../../state/settings/store.ts';

function notesGithubRepoFullName(url: string) {
  const m = String(url || '').match(/^https:\/\/github\.com\/([^/]+\/[^/]+)/i);
  return m ? m[1] : String(url || '');
}

function NotesConnectionItem({
  fullName,
  url,
  locked,
}: {
  fullName: string;
  url: string;
  locked: boolean;
}) {
  return (
    <div className="repo-list-item">
      <div className="repo-list-item-info">
        <div className="repo-list-item-name">{fullName}</div>
        <div className="repo-list-item-desc">{url}</div>
      </div>
      <div className="repo-list-item-actions">
        <a
          className="repo-list-item-link"
          href={url}
          target="_blank"
          rel="noopener noreferrer"
        >
          Link ↗
        </a>
        {locked ? null : (
          <button
            type="button"
            className="sediment-kb-delete-btn"
            id="btn-notes-connect-delete"
            onClick={() => {
              void deleteNotesGithubRepo();
            }}
          >
            Delete
          </button>
        )}
      </div>
    </div>
  );
}

export function NotesConnectionHost() {
  const { url, locked } = useSyncExternalStore(
    notesConnectionStore.subscribe,
    notesConnectionStore.getSnapshot,
  );
  if (!url) return null;
  return (
    <NotesConnectionItem
      fullName={notesGithubRepoFullName(url)}
      url={url}
      locked={locked}
    />
  );
}
