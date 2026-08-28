import { useEffect, useState, type CSSProperties } from 'react';
import { notifyState, useHostState } from '../state/host.ts';
import { addNoteLink, fetchAndCacheLinkTitle, removeNoteLink, resolveLinkTitle } from '../commands/links-bar.ts';

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
  const [adding, setAdding] = useState(false);
  const [confirmIndex, setConfirmIndex] = useState<number | null>(null);
  const [url, setUrl] = useState('');
  const [preview, setPreview] = useState('');
  const [resolvedTitle, setResolvedTitle] = useState('');

  useEffect(() => {
    setAdding(false);
    setConfirmIndex(null);
    setUrl('');
    setPreview('');
    setResolvedTitle('');
  }, [entry?.common_path]);

  useEffect(() => {
    if (!entry) return;
    for (const link of links) {
      if (link?.url && !host.index.titleFetchCache.has(link.url)) {
        void fetchAndCacheLinkTitle(link.url);
      }
    }
  }, [entry, linksKey, host.index.titleFetchCache]);

  useEffect(() => {
    const trimmed = url.trim();
    if (!adding || !trimmed) return undefined;
    const timer = setTimeout(async () => {
      setPreview('Fetching title…');
      const title = await resolveLinkTitle(trimmed);
      setResolvedTitle(title);
      setPreview(`→ 🔗 ${title} ↗`);
    }, 500);
    return () => clearTimeout(timer);
  }, [adding, url]);

  if (!entry || creating) {
    return <div id="md-links-bar" className="viewer-chrome-persisted" style={{ display: 'none', ...barStyle }} />;
  }

  async function onConfirmAdd() {
    const trimmed = url.trim();
    if (!trimmed) return;
    const result = await addNoteLink(trimmed, resolvedTitle);
    if (!result.ok) {
      setPreview(result.error === 'Link already exists' ? 'Link already exists' : `Error: ${result.error}`);
      return;
    }
    setAdding(false);
    setUrl('');
    setPreview('');
    setResolvedTitle('');
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
      {adding ? (
        <div className="links-input-row" style={{ display: 'flex', alignItems: 'center', gap: 6, marginTop: 4, width: '100%' }}>
          <input
            type="url"
            placeholder="Paste GitHub link…"
            value={url}
            onChange={(e) => {
              setUrl(e.target.value);
              setPreview('');
              setResolvedTitle('');
            }}
            style={{ flex: 1, fontSize: 12, padding: '3px 8px', border: '1px solid #d0d7de', borderRadius: 4 }}
            autoFocus
          />
          <button
            type="button"
            className="md-header-btn primary"
            data-action="confirm"
            style={{ fontSize: 11, padding: '2px 8px' }}
            onClick={() => void onConfirmAdd()}
          >
            Confirm
          </button>
          <button
            type="button"
            className="md-header-btn"
            data-action="cancel"
            style={{ fontSize: 11, padding: '2px 8px' }}
            onClick={() => {
              setAdding(false);
              setUrl('');
              setPreview('');
            }}
          >
            Cancel
          </button>
          <span data-role="preview" style={{ fontSize: 11, color: '#57606a' }}>
            {preview}
          </span>
        </div>
      ) : (
        <button
          type="button"
          className="md-header-btn"
          data-action="add-link"
          style={{ fontSize: 11, padding: '2px 8px', marginLeft: 'auto', flexShrink: 0 }}
          onClick={() => setAdding(true)}
        >
          ＋ Add link
        </button>
      )}
    </div>
  );
}
