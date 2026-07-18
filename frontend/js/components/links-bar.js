import { state, getEntryId } from '../state.js'
import * as api from '../api.js'

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

// ── confirmDeleteLink ──────────────────────────────────────────────────────

function confirmDeleteLink(entry, index, bar, wrapEl) {
  if (wrapEl.querySelector('.link-confirm-row')) return;
  const delBtn = wrapEl.querySelector('button');
  delBtn.style.display = 'none';

  const confirmRow = document.createElement('span');
  confirmRow.className = 'link-confirm-row';
  confirmRow.style.cssText = 'display:inline-flex;align-items:center;gap:4px;';

  const okBtn = document.createElement('button');
  okBtn.className = 'md-header-btn primary';
  okBtn.style.cssText = 'font-size:10px;padding:1px 6px;';
  okBtn.textContent = 'Confirm delete';

  const cancelBtn = document.createElement('button');
  cancelBtn.className = 'md-header-btn';
  cancelBtn.style.cssText = 'font-size:10px;padding:1px 6px;';
  cancelBtn.textContent = 'Cancel';

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

  confirmRow.append(okBtn, cancelBtn);
  wrapEl.appendChild(confirmRow);
}

// ── showAddLinkInput ───────────────────────────────────────────────────────

function showAddLinkInput(entry, bar) {
  if (bar.querySelector('.links-input-row')) return;
  const row = document.createElement('div');
  row.className = 'links-input-row';
  row.style.cssText = 'display:flex;align-items:center;gap:6px;margin-top:4px;width:100%;';

  const input = document.createElement('input');
  input.type = 'url';
  input.placeholder = 'Paste GitHub link…';
  input.style.cssText = 'flex:1;font-size:12px;padding:3px 8px;border:1px solid #d0d7de;border-radius:4px;';

  const confirmBtn = document.createElement('button');
  confirmBtn.className = 'md-header-btn primary';
  confirmBtn.style.cssText = 'font-size:11px;padding:2px 8px;';
  confirmBtn.textContent = 'Confirm';

  const cancelBtn = document.createElement('button');
  cancelBtn.className = 'md-header-btn';
  cancelBtn.style.cssText = 'font-size:11px;padding:2px 8px;';
  cancelBtn.textContent = 'Cancel';

  const preview = document.createElement('span');
  preview.style.cssText = 'font-size:11px;color:#57606a;';

  row.append(input, confirmBtn, cancelBtn, preview);
  bar.appendChild(row);
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

// ── renderLinksBar ─────────────────────────────────────────────────────────

export function renderLinksBar(entry) {
  const bar = document.getElementById('md-links-bar');
  bar.innerHTML = '';
  const links = entry && entry.links;

  const addBtn = document.createElement('button');
  addBtn.className = 'md-header-btn';
  addBtn.style.cssText = 'font-size:11px;padding:2px 8px;margin-left:auto;flex-shrink:0;';
  addBtn.textContent = '＋ Add link';
  addBtn.addEventListener('click', () => showAddLinkInput(entry, bar));

  if (!links || links.length === 0) {
    bar.style.display = 'flex';
    bar.appendChild(addBtn);
    return;
  }

  bar.style.display = 'flex';
  for (let i = 0; i < links.length; i++) {
    const link = links[i];
    const wrap = document.createElement('span');
    wrap.style.cssText = 'display:inline-flex;align-items:center;gap:4px;margin-right:12px;flex-shrink:0;';

    const a = document.createElement('a');
    a.href = link.url;
    a.target = '_blank';
    a.rel = 'noopener noreferrer';
    a.style.cssText = 'font-size:12px;color:#0969da;text-decoration:none;';
    a.textContent = '🔗 Loading…';
    fetchTitle(link.url).then(t => { a.textContent = `🔗 ${t} ↗`; });

    const delBtn = document.createElement('button');
    delBtn.style.cssText = 'font-size:10px;line-height:1;padding:1px 4px;border:1px solid #d0d7de;border-radius:3px;background:#fff;color:#8c959f;cursor:pointer;opacity:0.6;';
    delBtn.textContent = '×';
    delBtn.title = 'Delete this link';
    delBtn.addEventListener('click', () => confirmDeleteLink(entry, i, bar, wrap));

    wrap.append(a, delBtn);
    bar.appendChild(wrap);
  }
  bar.appendChild(addBtn);
}
