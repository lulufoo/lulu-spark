import { useEffect, useState, type CSSProperties } from 'react';
import { notifyState, useHostState } from '../state/host.ts';
import { fetchAndCacheLinkTitle, removeNoteLink } from '../commands/links-bar.ts';

const barStyle: CSSProperties = {
  padding: '8px 20px',
  borderBottom: '1px solid #d0d7de',
  alignItems: 'center',
  flexWrap: 'wrap',
  gap: '4px 0',
};

/** Tests / leftover callers: refresh the React bar. Production reads useHostState. */
export function renderLinksBar() {
  notifyState();
}

export function NotesLinksBar() {
  const host = useHostState();
  const entry = host.viewer.entry;
  const creating = Boolean(host.viewer.createSession);
  const links = entry?.links || [];
  const linksKey = links.map((l) => l.url).join('\0');
  const [confirmIndex, setConfirmIndex] = useState<number | null>(null);

  useEffect(() => {
    setConfirmIndex(null);
  }, [entry?.common_path]);

  useEffect(() => {
    if (!entry) return;
    for (const link of links) {
      if (link?.url && !host.index.titleFetchCache.has(link.url)) {
        void fetchAndCacheLinkTitle(link.url);
      }
    }
  }, [entry, linksKey, host.index.titleFetchCache]);

  if (!entry || creating || !links.length) {
    return <div id="md-links-bar" className="viewer-chrome-persisted" style={{ display: 'none', ...barStyle }} />;
  }

  async function onConfirmDelete(index: number) {
    const result = await removeNoteLink(index);
    if (!result.ok) {
      setConfirmIndex(null);
      return;
    }
    setConfirmIndex(null);
  }

  return (
    <div id="md-links-bar" className="viewer-chrome-persisted" style={{ display: 'flex', ...barStyle }}>
      {links.map((link, i) => {
        const title = host.index.titleFetchCache.get(link.url);
        return (
          <span
            key={`${link.url}-${i}`}
            data-link-index={i}
            style={{ display: 'inline-flex', alignItems: 'center', gap: 4, marginRight: 12, flexShrink: 0 }}
          >
            <a href={link.url} target="_blank" rel="noopener noreferrer" style={{ fontSize: 12, color: '#0969da', textDecoration: 'none' }}>
              {title ? `🔗 ${title} ↗` : '🔗 Loading…'}
            </a>
            {confirmIndex === i ? (
              <span className="link-confirm-row" style={{ display: 'inline-flex', alignItems: 'center', gap: 4 }}>
                <button
                  type="button"
                  className="md-header-btn primary"
                  data-action="confirm"
                  style={{ fontSize: 10, padding: '1px 6px' }}
                  onClick={() => void onConfirmDelete(i)}
                >
                  Confirm delete
                </button>
                <button
                  type="button"
                  className="md-header-btn"
                  data-action="cancel"
                  style={{ fontSize: 10, padding: '1px 6px' }}
                  onClick={() => setConfirmIndex(null)}
                >
                  Cancel
                </button>
              </span>
            ) : (
              <button
                type="button"
                title="Delete this link"
                data-action="delete-link"
                style={{
                  fontSize: 10,
                  lineHeight: 1,
                  padding: '1px 4px',
                  border: '1px solid #d0d7de',
                  borderRadius: 3,
                  background: '#fff',
                  color: '#8c959f',
                  cursor: 'pointer',
                  opacity: 0.6,
                }}
                onClick={() => setConfirmIndex(i)}
              >
                ×
              </button>
            )}
          </span>
        );
      })}
    </div>
  );
}
