import { state } from '../state.js'
import { LAYERS, IMPORTANCE_CYCLE } from '../constants.js'
import { escHtml, slugToTitle, filenameFromPath, topicFromPath, timeFromTs, importanceBadgeHtml } from '../utils.js'
import * as api from '../api.js'
import { openMoveProjectDialog } from './modals/move-project-dialog.js'

// ── Diff helpers ───────────────────────────────────────────────────────────

export function getEntryDiffState(entry) {
  let result = null;
  for (const layer of LAYERS) {
    if (!entry.layers?.includes(layer)) continue;
    const s = state.index.diffStatus.get(`${layer}/${entry.common_path}`);
    if (s === 'conflict') return 'conflict';
    if (s === 'modified') result = 'modified';
  }
  return result;
}

export function getLayerBadgeClass(layer, entry) {
  const s = state.index.diffStatus.get(`${layer}/${entry.common_path}`);
  if (s === 'conflict') return 'badge-conflict';
  if (s === 'modified') return 'badge-diff';
  return 'badge-layer';
}

// ── Badge listeners ────────────────────────────────────────────────────────

export function attachBadgeListeners(card, entry) {
  card.querySelectorAll('.badge[data-layer]').forEach(btn => {
    btn.addEventListener('click', () =>
      document.dispatchEvent(new CustomEvent('cta:open-entry', {
        detail: { common_path: entry.common_path, layer: btn.dataset.layer }
      }))
    );
  });
}

// ── buildCard ──────────────────────────────────────────────────────────────

export function buildCard(id, entry, title) {
  const card = document.createElement('div');
  card.className = 'doc-card';
  card.dataset.id = id;
  entry._id = id;  // attach id for later lookup

  const topic = topicFromPath(entry.common_path);
  const filename = filenameFromPath(entry.common_path);
  const displayTitle = title !== undefined ? title : null;
  const time = timeFromTs(entry.created_at);
  const projectDir = entry.common_path.split('/')[0];
  const topicDesc = state.index.topicDescriptions[projectDir] || '';

  const diffState = getEntryDiffState(entry);
  const dotHtml = diffState
    ? ` <span class="diff-dot ${diffState}">${diffState === 'conflict' ? '● conflict' : '● 待提交'}</span>`
    : '';

  const badgesHtml = LAYERS
    .filter(layer => entry.layers?.includes(layer))
    .map(layer => {
      const cc = entry._comment_counts?.[layer];
      const ccStr = cc ? `<span class="badge-comment-dot">${cc}</span>` : '';
      if (entry[`_unreachable_${layer}`]) {
        return `<span class="badge badge-unreachable" title="文件不可达">${layer}${ccStr}</span>`;
      }
      return `<button class="badge ${getLayerBadgeClass(layer, entry)}" data-layer="${layer}">${layer}${ccStr}</button>`;
    })
    .join('');

  const linkCount = entry.links && entry.links.length;
  const linksBadgeHtml = linkCount
    ? `<span class="badge badge-links" title="${linkCount} 个关联链接">👍 ×${linkCount}</span>`
    : '';

  const doneBadgeHtml = entry.done
    ? `<button class="badge badge-done" data-action="toggle-done" title="标记为未处理">✓ 已处理</button>`
    : `<button class="badge badge-done" data-action="toggle-done" title="标记为已处理">○ 处理</button>`;
  const moveBadgeHtml = `<button class="badge badge-move-project" data-action="move-project" title="移动到其他项目"><span class="move-icon">↳</span><span>移项</span></button>`;

  card.innerHTML = `
    <div class="doc-topic"${topicDesc ? ` data-tip="${escHtml(topicDesc)}"` : ''}>${topic}</div>
    <button class="doc-title-btn${displayTitle === null ? ' loading' : ''}">${displayTitle !== null ? escHtml(displayTitle) : ''}</button>
    <div class="doc-meta">${time}${dotHtml}</div>
    <div class="badges">${badgesHtml}${linksBadgeHtml}${importanceBadgeHtml(entry.importance)}${doneBadgeHtml}${moveBadgeHtml}</div>
  `;
  if (entry.done) card.classList.add('done');
  if (entry.importance) card.classList.add(`importance-${entry.importance}`);
  if (entry.common_path.split('/')[0] === 'inbox') card.classList.add('inbox-pending');
  const firstLayer = LAYERS.find(l => entry.layers?.includes(l)) || 'raw';
  card.querySelector('.doc-title-btn').addEventListener('click', () =>
    document.dispatchEvent(new CustomEvent('cta:open-entry', {
      detail: { common_path: entry.common_path, layer: firstLayer }
    }))
  );
  attachBadgeListeners(card, entry);
  card.querySelector('[data-action="toggle-done"]').addEventListener('click', () => toggleDone(entry, card));
  card.querySelector('[data-action="cycle-importance"]').addEventListener('click', () => cycleImportance(entry, card));
  card.querySelector('[data-action="move-project"]').addEventListener('click', () => openMoveProjectDialog(entry));
  return card;
}

// ── updateDiffInDOM ──────────────────────────────────────────────────────────

export async function cycleImportance(entry, card) {
  const cur = entry.importance;
  const idx = IMPORTANCE_CYCLE.indexOf(cur);
  const next = IMPORTANCE_CYCLE[(idx + 1) % IMPORTANCE_CYCLE.length];
  try {
    const data = await api.setImportance(entry.common_path, next);
    if (!data.ok) return;
    entry.importance = next;
    card.classList.remove('importance-high', 'importance-medium', 'importance-low');
    if (next) card.classList.add(`importance-${next}`);
    const btn = card.querySelector('[data-action="cycle-importance"]');
    if (btn) {
      btn.outerHTML = importanceBadgeHtml(next);
      card.querySelector('[data-action="cycle-importance"]').addEventListener('click', () => cycleImportance(entry, card));
    }
  } catch (e) {
    console.error('cycleImportance failed', e);
  }
}

// ── toggleDone ────────────────────────────────────────────────────────

export async function toggleDone(entry, card) {
  const newDone = !entry.done;
  try {
    const data = await api.setDone(entry.common_path, newDone);
    if (!data.ok) return;
    entry.done = newDone || undefined;
    card.classList.toggle('done', !!newDone);
    const btn = card.querySelector('[data-action="toggle-done"]');
    if (btn) {
      btn.textContent = newDone ? '✓ 已处理' : '○ 处理';
      btn.title = newDone ? '标记为未处理' : '标记为已处理';
    }
  } catch (e) {
    console.error('toggleDone failed', e);
  }
}

// ── renderDocList ──────────────────────────────────────────────────────────

export function renderDocList(entries, date) {
  const list = document.getElementById('doc-list');
  list.innerHTML = '';

  for (const { id, entry } of entries) {
    const card = buildCard(id, entry, state.index.titleCache.get(date)?.get(id));
    list.appendChild(card);
  }

  // Restore scroll position for this date
  const savedScroll = sessionStorage.getItem('cta_scroll_' + date);
  if (savedScroll) {
    requestAnimationFrame(() => { list.scrollTop = parseInt(savedScroll, 10); });
  }
}

// ── loadTitles ────────────────────────────────────────────────────────

export async function loadTitles(entries, date) {
  if (state.index.titleCache.has(date)) {
    updateTitlesInDOM(date);
    return;
  }

  state.index.titleCache.set(date, new Map());

  await Promise.all(entries.map(async ({ id, entry }) => {
    if (!entry.layers?.includes('raw')) {
      const title = slugToTitle(filenameFromPath(entry.common_path));
      state.index.titleCache.get(date).set(id, title);
      return;
    }

    try {
      const titlePath = entry.translations?.zh || entry.common_path;
      const text = await api.fetchFileContent('raw', titlePath);
      const h1Match = text.match(/^#\s+(.+)/m);
      const title = h1Match ? h1Match[1].trim() : slugToTitle(filenameFromPath(entry.common_path));
      state.index.titleCache.get(date).set(id, title);
    } catch {
      entry._unreachable_raw = true;
      state.index.titleCache.get(date).set(id, slugToTitle(filenameFromPath(entry.common_path)));
    }
  }));

  if (state.ui.activeDate === date) updateTitlesInDOM(date);
}

// ── updateTitlesInDOM ─────────────────────────────────────────────────

export function updateTitlesInDOM(date) {
  const cache = state.index.titleCache.get(date);
  if (!cache) return;
  const group = state.index.groupedByDate.find(g => g.date === date);
  if (!group) return;

  for (const { id, entry } of group.entries) {
    const card = document.querySelector(`.doc-card[data-id="${id}"]`);
    if (!card) continue;

    const titleEl = card.querySelector('.doc-title-btn');
    if (titleEl) {
      titleEl.classList.remove('loading');
      titleEl.textContent = cache.get(id) || slugToTitle(filenameFromPath(entry.common_path));
    }

    const badgesEl = card.querySelector('.badges');
    if (badgesEl) {
      const layerHtml = LAYERS
        .filter(layer => entry.layers?.includes(layer))
        .map(layer => {
          const cc = entry._comment_counts?.[layer];
          const ccStr = cc ? `<span class="badge-comment-dot">${cc}</span>` : '';
          if (entry[`_unreachable_${layer}`]) {
            return `<span class="badge badge-unreachable" title="文件不可达">${layer}${ccStr}</span>`;
          }
          return `<button class="badge ${getLayerBadgeClass(layer, entry)}" data-layer="${layer}">${layer}${ccStr}</button>`;
        })
        .join('');
      const linkCount = entry.links && entry.links.length;
      const linksBadgeHtml = linkCount
        ? `<span class="badge badge-links" title="${linkCount} 个关联链接">👍 ×${linkCount}</span>`
        : '';
      const doneBadgeHtml = entry.done
        ? `<button class="badge badge-done" data-action="toggle-done" title="标记为未处理">✓ 已处理</button>`
        : `<button class="badge badge-done" data-action="toggle-done" title="标记为已处理">○ 处理</button>`;
      const moveBadgeHtml = `<button class="badge badge-move-project" data-action="move-project" title="移动到其他项目"><span class="move-icon">↳</span><span>移项</span></button>`;
      badgesEl.innerHTML = layerHtml + linksBadgeHtml + importanceBadgeHtml(entry.importance) + doneBadgeHtml + moveBadgeHtml;
      attachBadgeListeners(card, entry);
      card.classList.toggle('done', !!entry.done);
      card.classList.remove('importance-high', 'importance-medium', 'importance-low');
      if (entry.importance) card.classList.add(`importance-${entry.importance}`);
      card.querySelector('[data-action="toggle-done"]').addEventListener('click', () => toggleDone(entry, card));
      card.querySelector('[data-action="cycle-importance"]').addEventListener('click', () => cycleImportance(entry, card));
      card.querySelector('[data-action="move-project"]').addEventListener('click', () => openMoveProjectDialog(entry));
    }
  }
}

export function updateDiffInDOM() {
  if (!state.ui.activeDate) return;
  const group = state.index.groupedByDate.find(g => g.date === state.ui.activeDate);
  if (!group) return;
  for (const { id, entry } of group.entries) {
    const card = document.querySelector(`.doc-card[data-id="${id}"]`);
    if (!card) continue;
    const metaEl = card.querySelector('.doc-meta');
    if (metaEl) {
      const diffState = getEntryDiffState(entry);
      const dotHtml = diffState
        ? ` <span class="diff-dot ${diffState}">${diffState === 'conflict' ? '● conflict' : '● 待提交'}</span>`
        : '';
      metaEl.innerHTML = timeFromTs(entry.created_at) + dotHtml;
    }
    const badgesEl = card.querySelector('.badges');
    if (badgesEl) {
      badgesEl.innerHTML = LAYERS
        .filter(layer => entry.layers?.includes(layer))
        .map(layer => {
          if (entry[`_unreachable_${layer}`]) {
            return `<span class="badge badge-unreachable" title="文件不可达">${layer}</span>`;
          }
          return `<button class="badge ${getLayerBadgeClass(layer, entry)}" data-layer="${layer}">${layer}</button>`;
        })
        .join('');
      attachBadgeListeners(card, entry);
    }
  }
}
