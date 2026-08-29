import { useSyncExternalStore } from 'react';
import { deleteWorkbenchGithubRepo } from '../../commands/settings/workbench-github.ts';
import { workbenchConnectionStore } from '../../state/settings/store.ts';

function workbenchGithubRepoFullName(url: string) {
  const m = String(url || '').match(/^https:\/\/github\.com\/([^/]+\/[^/]+)/i);
  return m ? m[1] : String(url || '');
}

function WorkbenchConnectionItem({
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
            id="btn-workbench-connect-delete"
            onClick={() => {
              void deleteWorkbenchGithubRepo();
            }}
          >
            Delete
          </button>
        )}
      </div>
    </div>
  );
}

export function WorkbenchConnectionHost() {
  const { url, locked } = useSyncExternalStore(
    workbenchConnectionStore.subscribe,
    workbenchConnectionStore.getSnapshot,
  );
  if (!url) return null;
  return (
    <WorkbenchConnectionItem
      fullName={workbenchGithubRepoFullName(url)}
      url={url}
      locked={locked}
    />
  );
}
