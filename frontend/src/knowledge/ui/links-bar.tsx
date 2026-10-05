import { useEffect, useState } from 'react';
import { notifyState, useHostState } from '../state/host.ts';
import { fetchAndCacheKbLinkTitle, removeKbLink } from '../commands/links-bar.ts';

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
  const [confirmIndex, setConfirmIndex] = useState<number | null>(null);

  useEffect(() => {
    setConfirmIndex(null);
  }, [host.viewer.kbRepo, host.viewer.kbPath]);

  useEffect(() => {
    if (!hasDoc) return;
    for (const link of links) {
      if (link?.url && !host.index.titleFetchCache.has(link.url)) {
        void fetchAndCacheKbLinkTitle(link.url);
      }
    }
  }, [hasDoc, linksKey, host.index.titleFetchCache]);

  if (!hasDoc || !links.length) {
    return (
      <div
        id="kb-md-links-bar"
        className="kb-reader-links-bar"
        style={{ display: 'none', padding: '8px 20px', borderBottom: '1px solid #d0d7de' }}
      />
    );
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
    </div>
  );
}
