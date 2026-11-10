import { defaultWorkspaceIconName, HomeNavIcon } from './nav-icons.tsx';
import { Icon } from '../../shared/icons/icon.tsx';
import { DEFAULT_PROJECT_ICON_ID, getIcon } from '../../shared/icons/index.ts';
import type { HomeProject } from '../state/store.ts';

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
  expandedWorkspaceIds?: string[];
  onOpenWorkspace: (id: string) => void;
  onOpenProject: (id: string, projectId: string) => void;
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

/** User-controlled groups; overflowing projects remain available through scrolling. */
export function HomeWorkspaceNav({
  workspaces,
  projects,
  unreadDocs,
  expandedWorkspaceIds = [],
  onOpenWorkspace,
  onOpenProject,
}: Props) {
  return (
    <div className="home-nav-workspaces" data-home-nav-list="workspace">
      {workspaces.map((row) => {
        const list = projects[row.id] ?? [];
        const shown = expandedWorkspaceIds.includes(row.id) ? list : [];
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
              </button>
            </div>
            {shown.map((project) => (
              <button
                key={project.id}
                type="button"
                className="home-chat-nav-item home-nav-project"
                data-home-project={project.id}
                onClick={() => onOpenProject(row.id, project.id)}
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
