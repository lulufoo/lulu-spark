import { useLayoutEffect, useMemo, useRef, useState } from 'react';
import { defaultWorkspaceIconName, HomeNavIcon } from './nav-icons.tsx';
import { Icon } from '../../shared/icons/icon.tsx';
import { DEFAULT_PROJECT_ICON_ID, getIcon } from '../../shared/icons/index.ts';
import { allocateWorkspaceRows } from '../state/workspace-rows.ts';
import type { HomeProject } from '../state/store.ts';

/** Used when `--home-nav-row-h` / `--home-nav-group-gap` cannot be read (the stylesheet is not applied). */
const FALLBACK_ROW_HEIGHT = 32;
const FALLBACK_GROUP_GAP = 8;

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

/** The hover text of the "…" mark: how many Projects the row leaves out. */
function hiddenTitle(count: number): string {
  return `${count} ${count === 1 ? 'project' : 'projects'} collapsed`;
}

function pixelsOf(el: HTMLElement, property: string, fallback: number): number {
  const raw = parseFloat(getComputedStyle(el).getPropertyValue(property));
  return Number.isFinite(raw) && raw >= 0 ? raw : fallback;
}

type Measure = {
  /** The list height in rows; fractional, since the gaps between workspaces are not whole rows. */
  rows: number;
  /** The gap above each workspace but the first, in rows. */
  gapRows: number;
};

/** The list's height and gap in rows; `null` until the list has been measured. */
function useMeasure(ref: React.RefObject<HTMLDivElement | null>): Measure | null {
  const [measure, setMeasure] = useState<Measure | null>(null);
  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return undefined;
    const update = () => {
      const rowHeight = pixelsOf(el, '--home-nav-row-h', FALLBACK_ROW_HEIGHT) || FALLBACK_ROW_HEIGHT;
      const gap = pixelsOf(el, '--home-nav-group-gap', FALLBACK_GROUP_GAP);
      const next = { rows: el.clientHeight / rowHeight, gapRows: gap / rowHeight };
      setMeasure((prev) =>
        prev && prev.rows === next.rows && prev.gapRows === next.gapRows ? prev : next,
      );
    };
    update();
    if (typeof ResizeObserver === 'undefined') return undefined;
    const observer = new ResizeObserver(update);
    observer.observe(el);
    return () => observer.disconnect();
  }, [ref]);
  return measure;
}

/**
 * Workspace rows, each with the projects the row allocation grants it (the order Settings
 * arranged) and a `…` mark (hover: "N projects collapsed") on the title row when some are cut off.
 * Every workspace but the first has a small gap above it (`--home-nav-group-gap`); the allocation
 * counts those gaps. The list fills the height the sidebar has left; when even the titles and
 * gaps do not fit it scrolls. Until the list is measured only the titles are shown.
 */
export function HomeWorkspaceNav({
  workspaces,
  projects,
  unreadDocs,
  onOpenWorkspace,
  onOpenProject,
}: Props) {
  const listRef = useRef<HTMLDivElement | null>(null);
  const measure = useMeasure(listRef);
  const counts = workspaces.map((row) => (projects[row.id] ?? []).length);
  const countKey = counts.join(',');
  const allocation = useMemo(
    () =>
      allocateWorkspaceRows(
        measure?.rows ?? 0,
        countKey ? countKey.split(',').map(Number) : [],
        measure?.gapRows ?? 0,
      ),
    [measure, countKey],
  );
  const measured = measure !== null;
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
                    data-home-hidden-count={plan.hidden}
                    title={hiddenTitle(plan.hidden)}
                    aria-label={hiddenTitle(plan.hidden)}
                  >
                    …
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
