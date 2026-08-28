import { renderToHtml } from '../../island.ts';

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
          <button type="button" className="sediment-kb-delete-btn" id="btn-notes-connect-delete">
            Delete
          </button>
        )}
      </div>
    </div>
  );
}

export function renderNotesConnectionHtml(fullName: string, url: string, locked: boolean) {
  return renderToHtml(<NotesConnectionItem fullName={fullName} url={url} locked={locked} />);
}
