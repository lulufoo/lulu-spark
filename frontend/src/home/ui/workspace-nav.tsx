import { useLayoutEffect, useMemo, useRef, useState } from 'react';
import { defaultWorkspaceIconName, HomeNavIcon } from './nav-icons.tsx';
import { Icon } from '../../shared/icons/icon.tsx';
import { DEFAULT_PROJECT_ICON_ID, getIcon } from '../../shared/icons/index.ts';
import { allocateWorkspaceRows } from '../state/workspace-rows.ts';
import type { HomeProject } from '../state/store.ts';

/** Used when `--home-nav-row-h` cannot be read (the stylesheet is not applied). */
const FALLBACK_ROW_HEIGHT = 32;

export type HomeWorkspaceRow = {
  id: string;
  name: string;
  isDefault: boolean;
  /** Id of the configured icon; empty = the default workspace icon. */
  icon: string;
};

type Props = {
  workspaces: HomeWorkspaceRow[];
  projects: Record<string, HomeProject[]>;
  /** Unread dot of the documents channel; shown on the built-in workspace. */
  unreadDocs: boolean;
  onOpenWorkspace: (id: string) => void;
  onOpenProject: (id: string, folder: string) => void;
};

/** `data-home-entry` of the two built-in workspaces keeps its old value; others use their id. */
function entryKey(row: HomeWorkspaceRow): string {
  if (row.isDefault) return 'spark';
  if (row.id === 'knowledge') return 'knowledge';
  return `workspace:${row.id}`;
}

/** The configured icon when it is a known one, otherwise the built-in default of that workspace. */
function WorkspaceIcon({ row }: { row: HomeWorkspaceRow }) {
  if (row.icon && getIcon(row.icon)) {
    return (
      <span className="home-desktop-shortcut-icon" data-home-icon="custom" aria-hidden="true">
        <Icon id={row.icon} size={18} />
      </span>
    );
  }
  return <HomeNavIcon name={defaultWorkspaceIconName(row.id)} />;
}

/** The sidebar's row height, from the `--home-nav-row-h` custom property. */
function rowHeightOf(el: HTMLElement): number {
  const raw = parseFloat(getComputedStyle(el).getPropertyValue('--home-nav-row-h'));
  return Number.isFinite(raw) && raw > 0 ? raw : FALLBACK_ROW_HEIGHT;
}

/** How many whole rows fit in the list; `null` until the list has been measured. */
function useAvailableRows(ref: React.RefObject<HTMLDivElement | null>): number | null {
  const [rows, setRows] = useState<number | null>(null);
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return undefined;
    const measure = () => setRows(Math.floor(el.clientHeight / rowHeightOf(el)));
    measure();
    if (typeof ResizeObserver === 'undefined') return undefined;
    const observer = new ResizeObserver(measure);
    observer.observe(el);
    return () => observer.disconnect();
  }, [ref]);
  return rows;
}

/**
 * Workspace rows, each with the projects the row allocation grants it (the order Settings
 * arranged) and a `+N` badge on the title row when some are cut off. The list fills the height
 * the sidebar has left; when even the titles do not fit it scrolls. Until the list is measured
 * only the titles are shown.
 */
export function HomeWorkspaceNav({
  workspaces,
  projects,
  unreadDocs,
  onOpenWorkspace,
  onOpenProject,
}: Props) {
  const listRef = useRef<HTMLDivElement | null>(null);
  const availableRows = useAvailableRows(listRef);
  const counts = workspaces.map((row) => (projects[row.id] ?? []).length);
  const countKey = counts.join(',');
  const allocation = useMemo(
    () => allocateWorkspaceRows(availableRows ?? 0, countKey ? countKey.split(',').map(Number) : []),
    [availableRows, countKey],
  );
  const measured = availableRows !== null;
  return (
    <div
      ref={listRef}
      className="home-nav-workspaces"
      data-home-nav-list="workspace"
      data-home-nav-overflow={allocation.overflow ? 'true' : 'false'}
    >
      {workspaces.map((row, index) => {
        const list = projects[row.id] ?? [];
        const plan = measured ? allocation.plans[index] : { shown: 0, hidden: 0 };
        const shown = list.slice(0, plan.shown);
        const hiddenNames = list.slice(plan.shown).map((project) => project.title);
        return (
          <div key={row.id} className="home-nav-workspace" data-home-workspace={row.id}>
            <div className="home-chat-nav-notes">
              <button
                type="button"
                className="home-chat-nav-item home-desktop-shortcut"
                data-home-entry={entryKey(row)}
                {...(row.isDefault && unreadDocs ? { 'data-home-unread': 'true' } : {})}
                onClick={() => onOpenWorkspace(row.id)}
              >
                <WorkspaceIcon row={row} />
                <span className="home-desktop-shortcut-label">{row.name}</span>
                {plan.hidden > 0 ? (
                  <span
                    className="home-nav-hidden-count"
                    data-home-hidden={row.id}
                    title={hiddenNames.join('\n')}
                  >
                    +{plan.hidden}
                  </span>
                ) : null}
              </button>
            </div>
            {shown.map((project) => (
              <button
                key={project.folder}
                type="button"
                className="home-chat-nav-item home-nav-project"
                data-home-project={project.folder}
                onClick={() => onOpenProject(row.id, project.folder)}
              >
                <span className="home-nav-project-icon" aria-hidden="true">
                  <Icon id={project.icon && getIcon(project.icon) ? project.icon : DEFAULT_PROJECT_ICON_ID} size={16} />
                </span>
                <span className="home-nav-project-label">{project.title}</span>
              </button>
            ))}
          </div>
        );
      })}
    </div>
  );
}
