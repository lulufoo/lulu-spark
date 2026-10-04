import { useSyncExternalStore } from 'react';
import { deleteSparkGithubRepo } from '../../commands/settings/spark-github.ts';
import { sparkConnectionStore } from '../../state/settings/store.ts';

function sparkGithubRepoFullName(url: string) {
  const m = String(url || '').match(/^https:\/\/github\.com\/([^/]+\/[^/]+)/i);
  return m ? m[1] : String(url || '');
}

function SparkConnectionItem({
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
            id="btn-spark-connect-delete"
            onClick={() => {
              void deleteSparkGithubRepo();
            }}
          >
            Delete
          </button>
        )}
      </div>
    </div>
  );
}

export function SparkConnectionHost() {
  const { url, locked } = useSyncExternalStore(
    sparkConnectionStore.subscribe,
    sparkConnectionStore.getSnapshot,
  );
  if (!url) return null;
  return (
    <SparkConnectionItem
      fullName={sparkGithubRepoFullName(url)}
      url={url}
      locked={locked}
    />
  );
}
