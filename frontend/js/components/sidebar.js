import { state } from '../state.js'
import { formatDate } from '../utils.js'
import { renderDocList, loadTitles } from './cards.js'

// ── buildGroups ────────────────────────────────────────────────────────────

export function buildGroups(indexData) {
  const map = new Map();
  for (const [id, entry] of Object.entries(indexData)) {
    if (!entry || typeof entry.created_at !== 'string') continue;
    const date = entry.created_at.slice(0, 8);
    if (!map.has(date)) map.set(date, []);
    map.get(date).push({ id, entry });
  }
  return [...map.entries()]
    .sort((a, b) => b[0].localeCompare(a[0]))
    .map(([date, entries]) => ({
      date,
      entries: entries.sort((a, b) => b.entry.created_at.localeCompare(a.entry.created_at))
    }));
}

// ── _renderTopicFilter ─────────────────────────────────────────────────

function _ensureSidebarZones(aside) {
  let inner = aside.querySelector('.sidebar-inner');
  let channelZone = aside.querySelector('#sidebar-channel-zone');
  let dateZone = aside.querySelector('#sidebar-date-zone');
  let resizer = aside.querySelector('#sidebar-resizer');

  if (!inner) {
    inner = document.createElement('div');
    inner.className = 'sidebar-inner';
    aside.appendChild(inner);
  }
  if (!channelZone) {
    channelZone = document.createElement('div');
    channelZone.id = 'sidebar-channel-zone';
    channelZone.className = 'sidebar-channel-zone';
    inner.appendChild(channelZone);
  }
  if (!dateZone) {
    dateZone = document.createElement('div');
    dateZone.id = 'sidebar-date-zone';
    dateZone.className = 'sidebar-date-zone';
    inner.appendChild(dateZone);
  }
  if (!resizer) {
    resizer = document.createElement('div');
    resizer.id = 'sidebar-resizer';
    resizer.className = 'sidebar-resizer';
    resizer.setAttribute('role', 'separator');
    resizer.setAttribute('aria-orientation', 'vertical');
    resizer.setAttribute('aria-label', '调整侧边栏宽度');
    resizer.tabIndex = 0;
    aside.appendChild(resizer);
  }

  return { channelZone, dateZone };
}

function _renderTopicFilter(parent) {
  const allEntries = Object.values(state.index.data || {});
  if (allEntries.length === 0) return;
  const topicCounts = {};
  for (const entry of allEntries) {
    const topic = entry.common_path?.split('/')[0] || 'unknown';
    topicCounts[topic] = (topicCounts[topic] || 0) + 1;
  }
  const total = allEntries.length;
  const topics = Object.keys(topicCounts).filter(t => t !== 'unknown').sort();

  const wrap = document.createElement('div');
  wrap.className = 'topic-filter';

  const sel = document.createElement('select');
  sel.className = 'topic-select';
  const optAll = document.createElement('option');
  optAll.value = '';
  optAll.textContent = `全部 (${total})`;
  sel.appendChild(optAll);
  const sep = document.createElement('option');
  sep.disabled = true;
  sep.textContent = '─────────';
  sel.appendChild(sep);
  for (const t of topics) {
    const opt = document.createElement('option');
    opt.value = t;
    opt.textContent = `${t} (${topicCounts[t]})`;
    sel.appendChild(opt);
  }
  if (topicCounts['unknown']) {
    const optUnknown = document.createElement('option');
    optUnknown.value = 'unknown';
    optUnknown.textContent = `unknown (${topicCounts['unknown']})`;
    optUnknown.title = 'common_path 格式异常的条目';
    sel.appendChild(optUnknown);
  }
  sel.value = state.ui.activeTopic || '';
  sel.addEventListener('change', (e) => selectTopic(e.target.value || null));
  wrap.appendChild(sel);

  const countEl = document.createElement('div');
  countEl.className = 'topic-count';
  if (state.ui.activeTopic) {
    countEl.textContent = `${topicCounts[state.ui.activeTopic] || 0} / ${total} 篇`;
    countEl.style.display = '';
  } else {
    countEl.style.display = 'none';
  }
  wrap.appendChild(countEl);
  parent.appendChild(wrap);
}

// ── renderSidebar ──────────────────────────────────────────────────────────

export function renderSidebar() {
  const aside = document.getElementById('sidebar');
  if (!aside) return;
  const { channelZone, dateZone } = _ensureSidebarZones(aside);
  channelZone.innerHTML = '';
  dateZone.innerHTML = '';
  _renderTopicFilter(channelZone);
  for (const { date, entries } of state.index.filteredGroups) {
    const d = formatDate(date);
    const tab = document.createElement('div');
    tab.className = 'date-tab';
    tab.dataset.date = date;
    tab.innerHTML = `
      <span class="day">${d.label}</span>
      <span class="month">${d.year} ${d.weekday}</span>
      <span class="count">${entries.length}</span>
    `;
    tab.addEventListener('click', () => selectDate(date));
    dateZone.appendChild(tab);
  }
}

// ── applyListFilters ───────────────────────────────────────────────────

export function applyListFilters() {
  let groups = state.index.groupedByDate;
  const topic = state.ui.activeTopic;
  if (topic) {
    groups = groups
      .map(({ date, entries }) => ({
        date,
        entries: entries.filter(({ entry }) =>
          (entry.common_path?.split('/')[0] || 'unknown') === topic
        ),
      }))
      .filter(({ entries }) => entries.length > 0);
  }
  const tagKey = state.ui.activeTagKey;
  if (tagKey) {
    groups = groups
      .map(({ date, entries }) => ({
        date,
        entries: entries.filter(({ entry }) => entry.tag_keys?.includes(tagKey)),
      }))
      .filter(({ entries }) => entries.length > 0);
  }
  state.index.filteredGroups = groups;
}

// ── renderTagFilterChip ────────────────────────────────────────────────────

export function renderTagFilterChip() {
  let chip = document.getElementById('tag-filter-chip');
  const key = state.ui.activeTagKey;
  if (!key) {
    if (chip) {
      if (typeof chip.remove === 'function') chip.remove();
      else chip.style.display = 'none';
    }
    return;
  }
  const reg = state.index.tagsRegistry?.keys?.[key];
  const label = reg?.value || key;
  if (!chip) {
    chip = document.createElement('span');
    chip.id = 'tag-filter-chip';
    chip.className = 'tag-filter-chip';
    const heading = document.getElementById('date-heading');
    if (heading?.insertAdjacentElement) heading.insertAdjacentElement('afterend', chip);
    else if (heading?.appendChild) heading.appendChild(chip);
  }
  chip.innerHTML = '';
  chip.style.display = '';
  const text = document.createElement('span');
  text.textContent = `标签：${label} `;
  const clearBtn = document.createElement('button');
  clearBtn.type = 'button';
  clearBtn.className = 'tag-filter-chip-clear';
  clearBtn.textContent = '×';
  clearBtn.title = '清除标签过滤';
  clearBtn.addEventListener('click', () => clearTagFilter());
  chip.append(text, clearBtn);
}

// ── selectTag / clearTagFilter ─────────────────────────────────────────────

export function selectTag(key) {
  if (state.ui.activeTagKey === key) {
    clearTagFilter();
    return;
  }
  state.ui.activeTagKey = key;
  _refreshFilteredList();
}

export function clearTagFilter() {
  state.ui.activeTagKey = null;
  _refreshFilteredList();
}

function _refreshFilteredList() {
  applyListFilters();
  renderSidebar();
  renderTagFilterChip();
  const list = document.getElementById('doc-list');
  if (state.index.filteredGroups.length > 0) {
    selectDate(state.index.filteredGroups[0].date);
  } else {
    state.ui.activeDate = null;
    document.getElementById('status').style.display = 'none';
    const heading = document.getElementById('date-heading');
    heading.style.display = '';
    heading.textContent = '无匹配条目';
    if (list) list.innerHTML = '<div class="empty-filter-msg">当前筛选条件下没有文档</div>';
  }
}

// ── selectDate ────────────────────────────────────────────────────────

export function selectDate(date) {
  state.ui.activeDate = date;
  sessionStorage.setItem('cta_active_date', date);

  document.querySelectorAll('.date-tab').forEach(t => {
    t.classList.toggle('active', t.dataset.date === date);
  });

  const group = state.index.filteredGroups.find(g => g.date === date);
  if (!group) return;

  const d = formatDate(date);
  document.getElementById('status').style.display = 'none';
  const heading = document.getElementById('date-heading');
  heading.style.display = '';
  heading.textContent = d.full + `  ·  ${group.entries.length} 篇`;
  renderTagFilterChip();

  renderDocList(group.entries, date);
  loadTitles(group.entries, date);
}

// ── selectTopic ────────────────────────────────────────────────────

export function selectTopic(key) {
  state.ui.activeTopic = key;
  applyListFilters();
  renderSidebar();
  renderTagFilterChip();
  if (state.index.filteredGroups.length > 0) {
    selectDate(state.index.filteredGroups[0].date);
  } else {
    state.ui.activeDate = null;
    document.getElementById('status').style.display = 'none';
    const heading = document.getElementById('date-heading');
    heading.style.display = '';
    heading.textContent = '无匹配条目';
    document.getElementById('doc-list').innerHTML =
      '<div class="empty-filter-msg">当前筛选条件下没有文档</div>';
  }
}
