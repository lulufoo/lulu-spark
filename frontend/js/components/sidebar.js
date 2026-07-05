import { state } from '../state.js'
import { formatDate } from '../utils.js'
import { renderDocList, loadTitles } from './cards.js'
import { closeFloatingListSelect, createFloatingListSelect } from './floating-list-select.js'

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

function _setSidebarChannelActive(channel) {
  document.querySelectorAll('.sidebar-channel-tab').forEach((tab) => {
    tab.classList.toggle('active', tab.dataset.channel === channel);
  });
}

function _renderChannelNav(parent) {
  const nav = document.createElement('div');
  nav.className = 'sidebar-channel-nav';

  const archiveTab = document.createElement('button');
  archiveTab.type = 'button';
  archiveTab.className = 'sidebar-channel-tab';
  archiveTab.dataset.channel = 'archive';
  archiveTab.textContent = '归档';
  archiveTab.addEventListener('click', () => selectArchiveChannel());

  nav.append(archiveTab);
  parent.appendChild(nav);
  _setSidebarChannelActive('archive');
}

export function selectArchiveChannel() {
  _setSidebarChannelActive('archive');
  if (state.index.filteredGroups.length > 0) {
    const date = state.ui.activeDate && state.index.filteredGroups.some((g) => g.date === state.ui.activeDate)
      ? state.ui.activeDate
      : state.index.filteredGroups[0].date;
    selectDate(date);
  } else {
    state.ui.activeDate = null;
    const status = document.getElementById('status');
    const heading = document.getElementById('date-heading');
    const list = document.getElementById('doc-list');
    if (status) status.style.display = '';
    if (heading) {
      heading.style.display = 'none';
      heading.textContent = '';
    }
    if (list) list.innerHTML = '';
  }
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

  /** @type {import('./floating-list-select.js').FloatingListSelectOption[]} */
  const options = [{ value: '', label: `全部 (${total})` }];
  for (const t of topics) {
    options.push({ value: t, label: `${t} (${topicCounts[t]})` });
  }
  if (topicCounts['unknown']) {
    options.push({
      value: 'unknown',
      label: `unknown (${topicCounts['unknown']})`,
      title: 'common_path 格式异常的条目',
    });
  }

  const { picker } = createFloatingListSelect({
    ariaLabel: '筛选主题',
    pickerClass: 'topic-select',
    value: state.ui.activeTopic || '',
    options,
    onSelect: (v) => selectTopic(v || null),
  });
  wrap.appendChild(picker);

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

function _entriesInTagCountScope() {
  const allEntries = Object.values(state.index.data || {});
  const topic = state.ui.activeTopic;
  if (!topic) return allEntries;
  return allEntries.filter(
    (entry) => (entry.common_path?.split('/')[0] || 'unknown') === topic
  );
}

function _tagLabel(key) {
  return state.index.tagsRegistry?.keys?.[key]?.value || key;
}

function _renderTagFilter(parent) {
  const scopeEntries = _entriesInTagCountScope();
  if (scopeEntries.length === 0) return;

  const tagCounts = {};
  for (const entry of scopeEntries) {
    for (const key of entry.tag_keys || []) {
      tagCounts[key] = (tagCounts[key] || 0) + 1;
    }
  }

  const total = scopeEntries.length;
  const activeKey = state.ui.activeTagKey;

  const wrap = document.createElement('div');
  wrap.className = 'tag-filter';

  /** @type {import('./floating-list-select.js').FloatingListSelectOption[]} */
  const options = [{ value: '', label: `全部 (${total})` }];
  const sortedKeys = Object.keys(tagCounts).sort((a, b) =>
    _tagLabel(a).localeCompare(_tagLabel(b))
  );
  for (const key of sortedKeys) {
    options.push({ value: key, label: `${_tagLabel(key)} (${tagCounts[key]})` });
  }
  if (activeKey && !tagCounts[activeKey]) {
    options.push({ value: activeKey, label: `${_tagLabel(activeKey)} (0)` });
  }

  const { picker } = createFloatingListSelect({
    ariaLabel: '筛选标签',
    pickerClass: 'tag-select',
    value: activeKey || '',
    options,
    onSelect: (v) => {
      if (v) selectTag(v);
      else clearTagFilter();
    },
  });
  wrap.appendChild(picker);

  const countEl = document.createElement('div');
  countEl.className = 'tag-count';
  if (activeKey) {
    countEl.textContent = `${tagCounts[activeKey] || 0} / ${total} 篇`;
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
  closeFloatingListSelect();
  const { channelZone, dateZone } = _ensureSidebarZones(aside);
  channelZone.innerHTML = '';
  dateZone.innerHTML = '';
  _renderChannelNav(channelZone);
  _renderTopicFilter(channelZone);
  _renderTagFilter(channelZone);
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
