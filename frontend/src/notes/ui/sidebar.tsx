import { useLayoutEffect, useRef } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { flushSync } from 'react-dom';
import { state, useHostState } from '../state/host.ts';
import { formatDate } from '../../shared/utils.ts';
import { closeFloatingListSelect, createFloatingListSelect } from '../../shared/floating-list-select.ts';
import { applyListFilters, buildGroups, selectDate, selectTopic } from '../commands/sidebar.ts';
import { isTimelineView } from '../commands/workspace.ts';
import { PageBackHome } from '../../shared/home-mark.tsx';
import { WindowDragStrip } from '../../shared/window-drag-strip.tsx';
import { DirTree } from './dir-tree.tsx';
import { renderProjectPicker } from './project-picker.ts';
import { attachNotesSidebarResize } from './sidebar-resize.ts';

export { applyListFilters, buildGroups, selectDate, selectTopic };

function _renderTopicFilter(parent: HTMLElement) {
  // Count what the list shows: every grouped document, so the picker and the list agree.
  const allEntries = (state.index.groupedByDate || []).flatMap((group) => group.entries.map(({ entry }) => entry));
  if (allEntries.length === 0) return;
  const topicCounts: Record<string, number> = {};
  for (const entry of allEntries) {
    const topic = entry.folder || 'unknown';
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
    const title = state.index.topicTitles[t];
    options.push({
      value: t,
      label: `${title && title !== t ? title : t} (${topicCounts[t]})`,
    });
  }
  if (topicCounts['unknown']) {
    options.push({
      value: 'unknown',
      label: `unknown (${topicCounts['unknown']})`,
      title: 'Entries with no category folder',
    });
  }

  const { picker } = createFloatingListSelect({
    ariaLabel: 'Filter by project',
    pickerClass: 'topic-select',
    value: state.ui.activeTopic || '',
    options,
    onSelect: (v: string) => selectTopic(v || null),
  });
  wrap.appendChild(picker);

  const countEl = document.createElement('div');
  countEl.className = 'topic-count';
  if (state.ui.activeTopic) {
    countEl.textContent = `${topicCounts[state.ui.activeTopic] || 0} / ${total}`;
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
  const workspaces = host.workspaces;
  const workspaceName = workspaces.list.find((w) => w.id === workspaces.current)?.name ?? '';
  const groups = host.index.filteredGroups || [];
  const activeDate = host.ui.activeDate;

  useLayoutEffect(() => {
    const parent = channelRef.current;
    if (!parent) return;
    const sidebar = parent.closest('#sidebar') || document.getElementById('sidebar');
    closeFloatingListSelect();
    sidebar?.querySelectorAll('.topic-filter').forEach((el) => el.remove());
    parent.replaceChildren();
    if (isTimelineView()) _renderTopicFilter(parent);
    else renderProjectPicker(parent);
    return () => {
      closeFloatingListSelect();
      parent.replaceChildren();
    };
  }, [data, topic, workspaces.current, workspaces.list, host.index.topicTitles]);

  useLayoutEffect(() => {
    const aside = document.getElementById('sidebar');
    if (!aside) return undefined;
    return attachNotesSidebarResize(aside);
  }, []);

  return (
    <>
      <div className="sidebar-inner">
        <WindowDragStrip>
          <PageBackHome />
        </WindowDragStrip>
        <div id="sidebar-workspace-title" className="sidebar-workspace-title" title={workspaceName}>
          {workspaceName}
        </div>
        {isTimelineView() ? (
          <div id="sidebar-channel-zone" className="sidebar-channel-zone" ref={channelRef} />
        ) : (
          <div id="sidebar-channel-zone" className="knowledge-doc-sidebar-header">
            <div className="knowledge-repo-picker-host" ref={channelRef} />
          </div>
        )}
        {isTimelineView() ? (
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
        ) : (
          <div id="sidebar-tree-zone" className="knowledge-doc-sidebar-tree">
            <DirTree />
          </div>
        )}
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
