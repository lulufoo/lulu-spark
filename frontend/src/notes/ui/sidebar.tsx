// @ts-nocheck — host/state snapshots stay unchecked; do not type this file alone.
import { useLayoutEffect, useRef } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { flushSync } from 'react-dom';
import { state, useHostState } from '../state/host.ts';
import { formatDate } from '../../shared/utils.ts';
import { closeFloatingListSelect, createFloatingListSelect } from '../../shared/floating-list-select.ts';
import {
  applyListFilters,
  buildGroups,
  clearTagFilter,
  renderTagFilterChip,
  selectDate,
  selectTag,
  selectTopic,
} from '../commands/sidebar.ts';

export {
  applyListFilters,
  buildGroups,
  clearTagFilter,
  renderTagFilterChip,
  selectDate,
  selectTag,
  selectTopic,
};

type NoteEntry = {
  common_path?: string | null;
  created_at?: string;
  tag_keys?: string[];
};

type Group = { date: string; entries: { id: string; entry: NoteEntry }[] };

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

export function NotesSidebar() {
  const host = useHostState();
  const channelRef = useRef<HTMLDivElement | null>(null);
  const data = host.index.data;
  const topic = host.ui.activeTopic;
  const tag = host.ui.activeTagKey;
  const registry = host.index.tagsRegistry;
  const groups = (host.index.filteredGroups || []) as Group[];
  const activeDate = host.ui.activeDate;

  useLayoutEffect(() => {
    const parent = channelRef.current;
    if (!parent) return;
    closeFloatingListSelect();
    parent.innerHTML = '';
    _renderTopicFilter(parent);
    _renderTagFilter(parent);
    return () => closeFloatingListSelect();
  }, [data, topic, tag, registry]);

  return (
    <>
      <div className="sidebar-inner">
        <div id="sidebar-channel-zone" className="sidebar-channel-zone" ref={channelRef} />
        <div id="sidebar-date-zone" className="sidebar-date-zone">
          {groups.map(({ date, entries }) => {
            const d = formatDate(date);
            return (
              <div
                key={date}
                className={`date-tab${activeDate === date ? ' active' : ''}`}
                data-date={date}
                onClick={() => selectDate(date)}
              >
                <DateTabInner label={d.label} year={d.year} weekday={d.weekday} count={entries.length} />
              </div>
            );
          })}
        </div>
      </div>
      <div
        id="sidebar-resizer"
        className="sidebar-resizer"
        role="separator"
        aria-orientation="vertical"
        aria-label="Resize sidebar"
        tabIndex={0}
      />
    </>
  );
}

let sidebarTestRoot: Root | null = null;

/** Tests only: mount NotesSidebar into `#sidebar`. Production uses ShellPages. */
export function renderSidebar() {
  const aside = document.getElementById('sidebar');
  if (!aside) return;
  closeFloatingListSelect();
  sidebarTestRoot?.unmount();
  sidebarTestRoot = createRoot(aside);
  flushSync(() => {
    sidebarTestRoot!.render(<NotesSidebar />);
  });
}
