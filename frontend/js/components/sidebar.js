import { state } from '../state.js'
import { formatDate } from '../utils.js'
import { renderDocList, loadTitles } from './cards.js'

// ── buildGroups ────────────────────────────────────────────────────────────

export function buildGroups(indexData) {
  const map = new Map();
  for (const [id, entry] of Object.entries(indexData)) {
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

function _renderTopicFilter(aside) {
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
  aside.appendChild(wrap);
}

// ── renderSidebar ──────────────────────────────────────────────────────────

export function renderSidebar() {
  const aside = document.getElementById('sidebar');
  aside.innerHTML = '';
  _renderTopicFilter(aside);
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
    aside.appendChild(tab);
  }
}

// ── selectDate ────────────────────────────────────────────────────────

export function selectDate(date) {
  state.ui.activeDate = date;
  sessionStorage.setItem('cta_active_date', date);

  // Update active tab
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

  renderDocList(group.entries, date);
  loadTitles(group.entries, date);
}

// ── selectTopic ────────────────────────────────────────────────────

export function selectTopic(key) {
  state.ui.activeTopic = key;
  if (!key) {
    state.index.filteredGroups = state.index.groupedByDate;
  } else {
    state.index.filteredGroups = state.index.groupedByDate
      .map(({ date, entries }) => ({
        date,
        entries: entries.filter(({ entry }) =>
          (entry.common_path?.split('/')[0] || 'unknown') === key
        ),
      }))
      .filter(({ entries }) => entries.length > 0);
  }
  renderSidebar();
  if (state.index.filteredGroups.length > 0) {
    selectDate(state.index.filteredGroups[0].date);
  }
}
