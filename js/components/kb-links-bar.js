import { state } from '../state.js'
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

// ── confirmDeleteKbLink ────────────────────────────────────────────────────

function confirmDeleteKbLink(index, bar, wrapEl) {
  if (wrapEl.querySelector('.link-confirm-row')) return;
  const delBtn = wrapEl.querySelector('button');
  delBtn.style.display = 'none';

  const confirmRow = document.createElement('span');
  confirmRow.className = 'link-confirm-row';
  confirmRow.style.cssText = 'display:inline-flex;align-items:center;gap:4px;';

  const okBtn = document.createElement('button');
  okBtn.className = 'md-header-btn primary';
  okBtn.style.cssText = 'font-size:10px;padding:1px 6px;';
  okBtn.textContent = '确认删除';

  const cancelBtn = document.createElement('button');
  cancelBtn.className = 'md-header-btn';
  cancelBtn.style.cssText = 'font-size:10px;padding:1px 6px;';
  cancelBtn.textContent = '取消';

  cancelBtn.addEventListener('click', () => {
    confirmRow.remove();
    delBtn.style.display = '';
  });

  okBtn.addEventListener('click', async () => {
    const { kbRepo, kbPath, annotation } = state.viewer;
    const newLinks = (annotation.links || []).filter((_, i) => i !== index);
    try {
      const data = await api.updateKbLinks(kbRepo, kbPath, newLinks);
      if (data.ok) {
        annotation.links = newLinks;
        renderKbLinksBar(annotation);
        document.dispatchEvent(new CustomEvent('kb:dirty', { detail: { msg: 'chore: update links' } }));
      } else {
        cancelBtn.click();
      }
    } catch { cancelBtn.click(); }
  });

  confirmRow.append(okBtn, cancelBtn);
  wrapEl.appendChild(confirmRow);
}

// ── showAddKbLinkInput ─────────────────────────────────────────────────────

function showAddKbLinkInput(bar) {
  if (bar.querySelector('.links-input-row')) return;
  const row = document.createElement('div');
  row.className = 'links-input-row';
  row.style.cssText = 'display:flex;align-items:center;gap:6px;margin-top:4px;width:100%;';

  const input = document.createElement('input');
  input.type = 'url';
  input.placeholder = '粘贴 GitHub 链接…';
  input.style.cssText = 'flex:1;font-size:12px;padding:3px 8px;border:1px solid #d0d7de;border-radius:4px;';

  const confirmBtn = document.createElement('button');
  confirmBtn.className = 'md-header-btn primary';
  confirmBtn.style.cssText = 'font-size:11px;padding:2px 8px;';
  confirmBtn.textContent = '确认';

  const cancelBtn = document.createElement('button');
  cancelBtn.className = 'md-header-btn';
  cancelBtn.style.cssText = 'font-size:11px;padding:2px 8px;';
  cancelBtn.textContent = '取消';

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
      preview.textContent = '获取标题中…';
      resolvedTitle = await fetchTitle(url);
      preview.textContent = `→ 🔗 ${resolvedTitle} ↗`;
    }, 500);
  });

  cancelBtn.addEventListener('click', () => row.remove());

  confirmBtn.addEventListener('click', async () => {
    const url = input.value.trim();
    if (!url) return;

    const { kbRepo, kbPath, annotation } = state.viewer;
    const existingLinks = annotation.links || [];
    if (existingLinks.some(l => l.url === url)) {
      preview.textContent = '链接已存在';
      return;
    }

    if (!resolvedTitle) resolvedTitle = await fetchTitle(url);
    const newLinks = [...existingLinks, { url }];

    try {
      const data = await api.updateKbLinks(kbRepo, kbPath, newLinks);
      if (data.ok) {
        annotation.links = newLinks;
        state.index.titleFetchCache.set(url, resolvedTitle);
        renderKbLinksBar(annotation);
        document.dispatchEvent(new CustomEvent('kb:dirty', { detail: { msg: 'chore: update links' } }));
      } else {
        preview.textContent = `错误：${data.error}`;
      }
    } catch (e) {
      preview.textContent = `错误：${e.message}`;
    }
  });
}

// ── renderKbLinksBar ───────────────────────────────────────────────────────

export function renderKbLinksBar(annotation) {
  const bar = document.getElementById('kb-md-links-bar');
  bar.innerHTML = '';
  const links = annotation && annotation.links;

  const addBtn = document.createElement('button');
  addBtn.className = 'md-header-btn';
  addBtn.style.cssText = 'font-size:11px;padding:2px 8px;margin-left:auto;flex-shrink:0;';
  addBtn.textContent = '＋ 添加链接';
  addBtn.addEventListener('click', () => showAddKbLinkInput(bar));

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
    a.textContent = '🔗 加载中…';
    fetchTitle(link.url).then(t => { a.textContent = `🔗 ${t} ↗`; });

    const delBtn = document.createElement('button');
    delBtn.className = 'md-header-btn';
    delBtn.style.cssText = 'font-size:10px;padding:1px 4px;color:#cf222e;';
    delBtn.textContent = '×';
    delBtn.addEventListener('click', () => confirmDeleteKbLink(i, bar, wrap));

    wrap.append(a, delBtn);
    bar.appendChild(wrap);
  }

  bar.appendChild(addBtn);
}
