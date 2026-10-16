import { useLayoutEffect, useRef } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { flushSync } from 'react-dom';
import { state, useHostState } from '../state/host.ts';
import { formatDate } from '../../shared/utils.ts';
import { closeFloatingListSelect, createFloatingListSelect } from '../../shared/floating-list-select.ts';
import { applyListFilters, buildGroups, selectDate, selectTopic } from '../commands/sidebar.ts';
import { isTimelineView } from '../commands/workspace.ts';
import { changeViewMode, switchWorkspace } from '../commands/workspace-switch.ts';
import { PageBackHome } from '../../shared/home-mark.tsx';
import { WindowDragStrip } from '../../shared/window-drag-strip.tsx';
import { attachNotesSidebarResize } from './sidebar-resize.ts';

export { applyListFilters, buildGroups, selectDate, selectTopic };

function _renderTopicFilter(parent: HTMLElement) {
  const allEntries = Object.values(state.index.data || {});
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

function _renderWorkspaceSwitch(parent: HTMLElement) {
  const { current, list } = state.workspaces;
  if (list.length === 0) return;
  const wrap = document.createElement('div');
  wrap.className = 'workspace-filter';

  const { picker } = createFloatingListSelect({
    ariaLabel: 'Select workspace',
    pickerClass: 'workspace-select',
    value: current,
    options: list.map((w) => ({ value: w.id, label: w.name })),
    onSelect: (id: string) => switchWorkspace(id),
  });
  wrap.appendChild(picker);

  const modes = document.createElement('div');
  modes.className = 'view-mode-switch';
  const active = isTimelineView() ? 'timeline' : 'directory';
  for (const mode of ['directory', 'timeline'] as const) {
    const button = document.createElement('button');
    button.type = 'button';
    button.className = `view-mode-btn${mode === active ? ' active' : ''}`;
    button.dataset.viewMode = mode;
    button.textContent = mode === 'directory' ? 'Directory' : 'Timeline';
    button.addEventListener('click', () => {
      void changeViewMode(mode).catch((err) => alert(err instanceof Error ? err.message : String(err)));
    });
    modes.appendChild(button);
  }
  wrap.appendChild(modes);
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
  const groups = host.index.filteredGroups || [];
  const activeDate = host.ui.activeDate;

  useLayoutEffect(() => {
    const parent = channelRef.current;
    if (!parent) return;
    const sidebar = parent.closest('#sidebar') || document.getElementById('sidebar');
    closeFloatingListSelect();
    sidebar?.querySelectorAll('.workspace-filter, .topic-filter').forEach((el) => el.remove());
    parent.replaceChildren();
    _renderWorkspaceSwitch(parent);
    if (!isTimelineView()) _renderTopicFilter(parent);
    return () => {
      closeFloatingListSelect();
      parent.replaceChildren();
    };
  }, [data, topic, workspaces.current, workspaces.list]);

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
