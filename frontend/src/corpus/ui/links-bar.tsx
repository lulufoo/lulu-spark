import { useEffect, useState } from 'react';
import { notifyState, useHostState } from '../state/host.ts';
import { addKbLink, fetchAndCacheKbLinkTitle, removeKbLink, resolveKbLinkTitle } from '../commands/links-bar.ts';

/** Tests / leftover callers: refresh the React bar. */
export function renderKbLinksBar() {
  notifyState();
}

export function KbLinksBar() {
  const host = useHostState();
  const annotation = host.viewer.annotation as { links?: { url: string }[] } | null;
  const links = annotation?.links || [];
  const linksKey = links.map((l) => l.url).join('\0');
  const hasDoc = Boolean(host.viewer.kbRepo && host.viewer.kbPath);
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
  }, [host.viewer.kbRepo, host.viewer.kbPath]);

  useEffect(() => {
    if (!hasDoc) return;
    for (const link of links) {
      if (link?.url && !host.index.titleFetchCache.has(link.url)) {
        void fetchAndCacheKbLinkTitle(link.url);
      }
    }
  }, [hasDoc, linksKey, host.index.titleFetchCache]);

  useEffect(() => {
    const trimmed = url.trim();
    if (!adding || !trimmed) return undefined;
    const timer = setTimeout(async () => {
      setPreview('Fetching title…');
      const title = await resolveKbLinkTitle(trimmed);
      setResolvedTitle(title);
      setPreview(`→ 🔗 ${title} ↗`);
    }, 500);
    return () => clearTimeout(timer);
  }, [adding, url]);

  if (!hasDoc) {
    return (
      <div
        id="kb-md-links-bar"
        className="kb-reader-links-bar"
        style={{ display: 'none', padding: '8px 20px', borderBottom: '1px solid #d0d7de' }}
      />
    );
  }

  async function onConfirmAdd() {
    const trimmed = url.trim();
    if (!trimmed) return;
    const result = await addKbLink(trimmed, resolvedTitle);
    if (!result.ok) {
      setPreview(result.error === 'Link already exists' ? 'Link already exists' : `Error: ${result.error}`);
      return;
    }
    setAdding(false);
    setUrl('');
    setPreview('');
  }

  return (
    <div
      id="kb-md-links-bar"
      className="kb-reader-links-bar"
      style={{ display: 'flex', padding: '8px 20px', borderBottom: '1px solid #d0d7de', alignItems: 'center', flexWrap: 'wrap' }}
    >
      {links.map((link, i) => {
        const title = host.index.titleFetchCache.get(link.url);
        return (
          <span
            key={`${link.url}-${i}`}
            className="kb-link-wrap"
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
                  style={{ fontSize: 10, padding: '1px 6px' }}
                  onClick={() => void removeKbLink(i).then(() => setConfirmIndex(null))}
                >
                  Confirm delete
                </button>
                <button type="button" className="md-header-btn" style={{ fontSize: 10, padding: '1px 6px' }} onClick={() => setConfirmIndex(null)}>
                  Cancel
                </button>
              </span>
            ) : (
              <button
                type="button"
                className="md-header-btn"
                data-link-index={i}
                style={{ fontSize: 10, padding: '1px 4px', color: '#cf222e' }}
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
          <button type="button" className="md-header-btn primary" style={{ fontSize: 11, padding: '2px 8px' }} onClick={() => void onConfirmAdd()}>
            Confirm
          </button>
          <button
            type="button"
            className="md-header-btn"
            style={{ fontSize: 11, padding: '2px 8px' }}
            onClick={() => {
              setAdding(false);
              setUrl('');
              setPreview('');
            }}
          >
            Cancel
          </button>
          <span style={{ fontSize: 11, color: '#57606a' }}>{preview}</span>
        </div>
      ) : (
        <button
          type="button"
          className="md-header-btn kb-link-add"
          style={{ fontSize: 11, padding: '2px 8px', marginLeft: 'auto', flexShrink: 0 }}
          onClick={() => setAdding(true)}
        >
          ＋ Add link
        </button>
      )}
    </div>
  );
}
