// @ts-nocheck — ported from JS; state shapes stay unchecked like checkJs:false.
import { state, getEntryId } from '../host/state.ts'
import * as api from '../host/api.ts'
import { renderToHtml } from '../island.ts';

// ── fetchTitle / fallbackTitle ─────────────────────────────────────────────

function fallbackTitle(url) {
  try {
    return decodeURIComponent(new URL(url).pathname.split('/').filter(Boolean).pop()).replace(/\.md$/, '');
  } catch { return url; }
}

async function fetchTitle(url) {
  if (state.index.titleFetchCache.has(url)) return state.index.titleFetchCache.get(url);
  try {
    const data = await api.fetchLinkTitle(url);
    const title = data.title || fallbackTitle(url);
    state.index.titleFetchCache.set(url, title);
    return title;
  } catch {
    return fallbackTitle(url);
  }
}

// ── updateCardLinksBadge ───────────────────────────────────────────────────

function updateCardLinksBadge(entry) {
  const id = getEntryId(entry);
  if (!id) return;
  const card = document.querySelector(`.doc-card[data-id="${id}"]`);
  if (!card) return;
  const badges = card.querySelector('.badges');
  if (!badges) return;
  let badge = badges.querySelector('.badge-links');
  const count = entry.links && entry.links.length;
  if (count) {
    if (!badge) {
      badge = document.createElement('span');
      badge.className = 'badge badge-links';
      badges.appendChild(badge);
    }
    badge.title = `${count} linked items`;
    badge.textContent = `👍 ×${count}`;
  } else {
    if (badge) badge.remove();
  }
}

function ConfirmDeleteRow() {
  return (
    <span className="link-confirm-row" style={{ display: 'inline-flex', alignItems: 'center', gap: 4 }}>
      <button className="md-header-btn primary" data-action="confirm" style={{ fontSize: 10, padding: '1px 6px' }}>
        Confirm delete
      </button>
      <button className="md-header-btn" data-action="cancel" style={{ fontSize: 10, padding: '1px 6px' }}>
        Cancel
      </button>
    </span>
  );
}

// ── confirmDeleteLink ──────────────────────────────────────────────────────

function confirmDeleteLink(entry, index, bar, wrapEl) {
  if (wrapEl.querySelector('.link-confirm-row')) return;
  const delBtn = wrapEl.querySelector('button');
  delBtn.style.display = 'none';

  wrapEl.insertAdjacentHTML('beforeend', renderToHtml(<ConfirmDeleteRow />));
  const confirmRow = wrapEl.querySelector('.link-confirm-row');
  const okBtn = confirmRow.querySelector('[data-action="confirm"]');
  const cancelBtn = confirmRow.querySelector('[data-action="cancel"]');

  cancelBtn.addEventListener('click', () => {
    confirmRow.remove();
    delBtn.style.display = '';
  });

  okBtn.addEventListener('click', async () => {
    const newLinks = (entry.links || []).filter((_, i) => i !== index);
    try {
      const data = await api.updateLinks(entry.common_path, newLinks);
      if (data.ok) {
        entry.links = newLinks;
        updateCardLinksBadge(entry);
        renderLinksBar(entry);
      } else {
        cancelBtn.click();
      }
    } catch { cancelBtn.click(); }
  });
}

function AddLinkRow() {
  return (
    <div
      className="links-input-row"
      style={{ display: 'flex', alignItems: 'center', gap: 6, marginTop: 4, width: '100%' }}
    >
      <input
        type="url"
        placeholder="Paste GitHub link…"
        style={{ flex: 1, fontSize: 12, padding: '3px 8px', border: '1px solid #d0d7de', borderRadius: 4 }}
      />
      <button className="md-header-btn primary" data-action="confirm" style={{ fontSize: 11, padding: '2px 8px' }}>
        Confirm
      </button>
      <button className="md-header-btn" data-action="cancel" style={{ fontSize: 11, padding: '2px 8px' }}>
        Cancel
      </button>
      <span data-role="preview" style={{ fontSize: 11, color: '#57606a' }} />
    </div>
  );
}

// ── showAddLinkInput ───────────────────────────────────────────────────────

function showAddLinkInput(entry, bar) {
  if (bar.querySelector('.links-input-row')) return;
  bar.insertAdjacentHTML('beforeend', renderToHtml(<AddLinkRow />));
  const row = bar.querySelector('.links-input-row');
  const input = row.querySelector('input');
  const confirmBtn = row.querySelector('[data-action="confirm"]');
  const cancelBtn = row.querySelector('[data-action="cancel"]');
  const preview = row.querySelector('[data-role="preview"]');
  input.focus();

  let resolvedTitle = '';
  let fetchTimer = null;
  input.addEventListener('input', () => {
    clearTimeout(fetchTimer);
    preview.textContent = '';
    resolvedTitle = '';
    const url = input.value.trim();
    if (!url) return;
    fetchTimer = setTimeout(async () => {
      preview.textContent = 'Fetching title…';
      resolvedTitle = await fetchTitle(url);
      preview.textContent = `→ 🔗 ${resolvedTitle} ↗`;
    }, 500);
  });

  cancelBtn.addEventListener('click', () => row.remove());

  confirmBtn.addEventListener('click', async () => {
    const url = input.value.trim();
    if (!url) return;

    const existingLinks = entry.links || [];
    if (existingLinks.some(l => l.url === url)) {
      preview.textContent = 'Link already exists';
      return;
    }

    if (!resolvedTitle) resolvedTitle = await fetchTitle(url);
    const newLinks = [...existingLinks, { url }];

    try {
      const data = await api.updateLinks(entry.common_path, newLinks);
      if (data.ok) {
        entry.links = newLinks;
        state.index.titleFetchCache.set(url, resolvedTitle);
        updateCardLinksBadge(entry);
        renderLinksBar(entry);
      } else {
        preview.textContent = `Error: ${data.error}`;
      }
    } catch (e) {
      preview.textContent = `Error: ${e.message}`;
    }
  });
}

function AddLinkBtn() {
  return (
    <button
      className="md-header-btn"
      data-action="add-link"
      style={{ fontSize: 11, padding: '2px 8px', marginLeft: 'auto', flexShrink: 0 }}
    >
      ＋ Add link
    </button>
  );
}

function LinksBar({ links }) {
  return (
    <>
      {links.map((link, i) => (
        <span
          key={`${link.url}-${i}`}
          data-link-index={i}
          style={{ display: 'inline-flex', alignItems: 'center', gap: 4, marginRight: 12, flexShrink: 0 }}
        >
          <a
            href={link.url}
            target="_blank"
            rel="noopener noreferrer"
            style={{ fontSize: 12, color: '#0969da', textDecoration: 'none' }}
          >
            🔗 Loading…
          </a>
          <button
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
          >
            ×
          </button>
        </span>
      ))}
      <AddLinkBtn />
    </>
  );
}

// ── renderLinksBar ─────────────────────────────────────────────────────────

export function renderLinksBar(entry) {
  const bar = document.getElementById('md-links-bar');
  bar.innerHTML = '';
  const links = entry && entry.links;

  if (!links || links.length === 0) {
    bar.style.display = 'flex';
    bar.innerHTML = renderToHtml(<AddLinkBtn />);
    bar.querySelector('[data-action="add-link"]')?.addEventListener('click', () => showAddLinkInput(entry, bar));
    return;
  }

  bar.style.display = 'flex';
  bar.innerHTML = renderToHtml(<LinksBar links={links} />);
  bar.querySelectorAll('[data-link-index]').forEach((wrap) => {
    const i = Number(wrap.dataset.linkIndex);
    const link = links[i];
    const a = wrap.querySelector('a');
    if (a && link) {
      fetchTitle(link.url).then((t) => { a.textContent = `🔗 ${t} ↗`; });
    }
    wrap.querySelector('[data-action="delete-link"]')?.addEventListener('click', () => {
      confirmDeleteLink(entry, i, bar, wrap);
    });
  });
  bar.querySelector('[data-action="add-link"]')?.addEventListener('click', () => showAddLinkInput(entry, bar));
}
