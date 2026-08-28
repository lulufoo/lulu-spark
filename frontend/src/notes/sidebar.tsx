// @ts-nocheck — ported from JS; state shapes stay unchecked like checkJs:false.
import { state } from '../host/state.ts';
import { formatDate } from '../shared/utils.ts';
import { renderDocList, loadTitles } from './cards.tsx';
import { closeFloatingListSelect, createFloatingListSelect } from '../shared/floating-list-select.ts';
import { parseHash, navigateToDateList } from '../router/index.ts';
import { renderToHtml } from '../island.ts';

type NoteEntry = {
  common_path?: string | null;
  created_at?: string;
  tag_keys?: string[];
};

type Group = { date: string; entries: { id: string; entry: NoteEntry }[] };

export function buildGroups(indexData: Record<string, NoteEntry>) {
  const map = new Map<string, { id: string; entry: NoteEntry }[]>();
  for (const [id, entry] of Object.entries(indexData)) {
    if (!entry || typeof entry.created_at !== 'string') continue;
    const date = entry.created_at.slice(0, 8);
    if (!map.has(date)) map.set(date, []);
    map.get(date)!.push({ id, entry });
  }
  return [...map.entries()]
    .sort((a, b) => b[0].localeCompare(a[0]))
    .map(([date, entries]) => ({
      date,
      entries: entries.sort((a, b) => String(b.entry.created_at).localeCompare(String(a.entry.created_at))),
    }));
}

function _ensureSidebarZones(aside: HTMLElement) {
  let inner = aside.querySelector('.sidebar-inner');
  let channelZone = aside.querySelector('#sidebar-channel-zone') as HTMLElement | null;
  let dateZone = aside.querySelector('#sidebar-date-zone') as HTMLElement | null;
  let resizer = aside.querySelector('#sidebar-resizer') as HTMLElement | null;

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
    resizer.setAttribute('aria-label', 'Resize sidebar');
    resizer.tabIndex = 0;
    aside.appendChild(resizer);
  }

  return { channelZone, dateZone };
}

function _renderTopicFilter(parent: HTMLElement) {
  const allEntries = Object.values((state.index.data || {}) as Record<string, NoteEntry>);
  if (allEntries.length === 0) return;
  const topicCounts: Record<string, number> = {};
  for (const entry of allEntries) {
    const topic = entry.common_path?.split('/')[0] || 'unknown';
    topicCounts[topic] = (topicCounts[topic] || 0) + 1;
  }
  const total = allEntries.length;
  const topics = Object.keys(topicCounts)
    .filter((t) => t !== 'unknown')
    .sort();

  const wrap = document.createElement('div');
  wrap.className = 'topic-filter';

  const options: { value: string; label: string; title?: string }[] = [{ value: '', label: `All (${total})` }];
  for (const t of topics) {
    options.push({ value: t, label: `${t} (${topicCounts[t]})` });
  }
  if (topicCounts['unknown']) {
    options.push({
      value: 'unknown',
      label: `unknown (${topicCounts['unknown']})`,
      title: 'Entries with invalid common_path',
    });
  }

  const { picker } = createFloatingListSelect({
    ariaLabel: 'Filter by topic',
    pickerClass: 'topic-select',
    value: state.ui.activeTopic || '',
    options,
    onSelect: (v: string) => selectTopic(v || null),
  });
  wrap.appendChild(picker);

  const countEl = document.createElement('div');
  countEl.className = 'topic-count';
  if (state.ui.activeTopic) {
    countEl.textContent = `${topicCounts[state.ui.activeTopic] || 0} / ${total} items`;
    countEl.style.display = '';
  } else {
    countEl.style.display = 'none';
  }
  wrap.appendChild(countEl);
  parent.appendChild(wrap);
}

function _entriesInTagCountScope() {
  const allEntries = Object.values((state.index.data || {}) as Record<string, NoteEntry>);
  const topic = state.ui.activeTopic;
  if (!topic) return allEntries;
  return allEntries.filter((entry) => (entry.common_path?.split('/')[0] || 'unknown') === topic);
}

function _tagLabel(key: string) {
  return state.index.tagsRegistry?.keys?.[key]?.value || key;
}

function _renderTagFilter(parent: HTMLElement) {
  const scopeEntries = _entriesInTagCountScope();
  if (scopeEntries.length === 0) return;

  const tagCounts: Record<string, number> = {};
  for (const entry of scopeEntries) {
    for (const key of entry.tag_keys || []) {
      tagCounts[key] = (tagCounts[key] || 0) + 1;
    }
  }

  const total = scopeEntries.length;
  const activeKey = state.ui.activeTagKey;

  const wrap = document.createElement('div');
  wrap.className = 'tag-filter';

  const options: { value: string; label: string }[] = [{ value: '', label: `All (${total})` }];
  const sortedKeys = Object.keys(tagCounts).sort((a, b) => _tagLabel(a).localeCompare(_tagLabel(b)));
  for (const key of sortedKeys) {
    options.push({ value: key, label: `${_tagLabel(key)} (${tagCounts[key]})` });
  }
  if (activeKey && !tagCounts[activeKey]) {
    options.push({ value: activeKey, label: `${_tagLabel(activeKey)} (0)` });
  }

  const { picker } = createFloatingListSelect({
    ariaLabel: 'Filter by tag',
    pickerClass: 'tag-select',
    value: activeKey || '',
    options,
    onSelect: (v: string) => {
      if (v) selectTag(v);
      else clearTagFilter();
    },
  });
  wrap.appendChild(picker);

  const countEl = document.createElement('div');
  countEl.className = 'tag-count';
  if (activeKey) {
    countEl.textContent = `${tagCounts[activeKey] || 0} / ${total} items`;
    countEl.style.display = '';
  } else {
    countEl.style.display = 'none';
  }
  wrap.appendChild(countEl);
  parent.appendChild(wrap);
}

function DateTabInner({
  label,
  year,
  weekday,
  count,
}: {
  label: string;
  year: string;
  weekday: string;
  count: number;
}) {
  return (
    <>
      <span className="day">{label}</span>
      <span className="month">
        {year} {weekday}
      </span>
      <span className="count">{count}</span>
    </>
  );
}

export function renderSidebar() {
  const aside = document.getElementById('sidebar');
  if (!aside) return;
  closeFloatingListSelect();
  const { channelZone, dateZone } = _ensureSidebarZones(aside);
  channelZone.innerHTML = '';
  dateZone.innerHTML = '';
  _renderTopicFilter(channelZone);
  _renderTagFilter(channelZone);
  for (const { date, entries } of state.index.filteredGroups as Group[]) {
    const d = formatDate(date);
    const tab = document.createElement('div');
    tab.className = 'date-tab';
    tab.dataset.date = date;
    tab.innerHTML = renderToHtml(
      <DateTabInner label={d.label} year={d.year} weekday={d.weekday} count={entries.length} />,
    );
    tab.addEventListener('click', () => selectDate(date));
    dateZone.appendChild(tab);
  }
}

export function applyListFilters() {
  let groups = state.index.groupedByDate as Group[];
  const topic = state.ui.activeTopic;
  if (topic) {
    groups = groups
      .map(({ date, entries }) => ({
        date,
        entries: entries.filter(({ entry }) => (entry.common_path?.split('/')[0] || 'unknown') === topic),
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

function EmptyFilterMsg() {
  return <div className="empty-filter-msg">No documents match the current filters</div>;
}

function TagFilterChipContent({ label }: { label: string }) {
  return (
    <>
      <span>{`Tag: ${label} `}</span>
      <button type="button" className="tag-filter-chip-clear" title="Clear tag filter">
        ×
      </button>
    </>
  );
}

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
  chip.innerHTML = renderToHtml(<TagFilterChipContent label={label} />);
  chip.style.display = '';
  chip.querySelector('.tag-filter-chip-clear')?.addEventListener('click', () => clearTagFilter());
}

export function selectTag(key: string) {
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
    const status = document.getElementById('status');
    if (status) status.style.display = 'none';
    const heading = document.getElementById('date-heading');
    if (heading) {
      heading.style.display = '';
      heading.textContent = 'No matching items';
    }
    if (list) list.innerHTML = renderToHtml(<EmptyFilterMsg />);
  }
}

export function selectDate(date: string) {
  const route = parseHash();
  if (route.name === 'workbench') {
    const hasNote = Boolean(route.params?.note);
    const creating = Boolean(state.viewer?.createSession);
    if (hasNote || creating) {
      if (creating) state.viewer.createSession = null;
      navigateToDateList(date);
      return;
    }
  }

  state.ui.activeDate = date;
  sessionStorage.setItem('cta_active_date', date);

  document.querySelectorAll('.date-tab').forEach((t) => {
    t.classList.toggle('active', (t as HTMLElement).dataset.date === date);
  });

  const group = (state.index.filteredGroups as Group[]).find((g) => g.date === date);
  if (!group) return;

  const d = formatDate(date);
  const status = document.getElementById('status');
  if (status) status.style.display = 'none';
  const heading = document.getElementById('date-heading');
  if (heading) {
    heading.style.display = '';
    heading.textContent = d.full + `  ·  ${group.entries.length} items`;
  }
  renderTagFilterChip();

  renderDocList(group.entries, date);
  loadTitles(group.entries, date);
}

export function selectTopic(key: string | null) {
  state.ui.activeTopic = key;
  applyListFilters();
  renderSidebar();
  renderTagFilterChip();
  if (state.index.filteredGroups.length > 0) {
    selectDate(state.index.filteredGroups[0].date);
  } else {
    state.ui.activeDate = null;
    const status = document.getElementById('status');
    if (status) status.style.display = 'none';
    const heading = document.getElementById('date-heading');
    if (heading) {
      heading.style.display = '';
      heading.textContent = 'No matching items';
    }
    const list = document.getElementById('doc-list');
    if (list) list.innerHTML = renderToHtml(<EmptyFilterMsg />);
  }
}
