import { HomeNavIcon } from './nav-icons.tsx';
import { HOME_PROJECT_LIMIT } from '../commands/workspace-nav.ts';
import type { HomeProject } from '../state/store.ts';

export type HomeWorkspaceRow = { id: string; name: string; isDefault: boolean };

type Props = {
  workspaces: HomeWorkspaceRow[];
  projects: Record<string, HomeProject[]>;
  /** Unread dot of the documents channel; shown on the built-in workspace. */
  unreadDocs: boolean;
  onOpenWorkspace: (id: string) => void;
  onOpenProject: (id: string, folder: string) => void;
  onCreateDocument: () => void;
};

/** `data-home-entry` of the two built-in workspaces keeps its old value; others use their id. */
function entryKey(row: HomeWorkspaceRow): string {
  if (row.isDefault) return 'spark';
  if (row.id === 'knowledge') return 'knowledge';
  return `workspace:${row.id}`;
}

function PlusIcon() {
  return (
    <svg className="home-chat-new-icon" viewBox="0 0 24 24" aria-hidden="true" focusable="false">
      <path
        fill="currentColor"
        d="M11 5a1 1 0 1 1 2 0v6h6a1 1 0 1 1 0 2h-6v6a1 1 0 1 1-2 0v-6H5a1 1 0 1 1 0-2h6V5z"
      />
    </svg>
  );
}

/** Workspace rows, each with its first projects (title order) and `...` when there are more. */
export function HomeWorkspaceNav({
  workspaces,
  projects,
  unreadDocs,
  onOpenWorkspace,
  onOpenProject,
  onCreateDocument,
}: Props) {
  return (
    <>
      {workspaces.map((row) => {
        const list = projects[row.id] ?? [];
        const shown = list.slice(0, HOME_PROJECT_LIMIT);
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
                <HomeNavIcon name={row.id === 'knowledge' ? 'knowledge' : 'notes'} />
                <span className="home-desktop-shortcut-label">{row.name}</span>
              </button>
              {row.isDefault ? (
                <button
                  type="button"
                  className="home-chat-new"
                  data-role="create-note"
                  aria-label="New document"
                  title="New document"
                  onClick={onCreateDocument}
                >
                  <PlusIcon />
                </button>
              ) : null}
            </div>
            {shown.map((project) => (
              <button
                key={project.folder}
                type="button"
                className="home-chat-nav-item home-nav-project"
                data-home-project={project.folder}
                onClick={() => onOpenProject(row.id, project.folder)}
              >
                <span className="home-nav-project-label">{project.title}</span>
              </button>
            ))}
            {list.length > HOME_PROJECT_LIMIT ? (
              <button
                type="button"
                className="home-chat-nav-item home-nav-project home-nav-more"
                data-home-more={row.id}
                aria-label={`All projects of ${row.name}`}
                onClick={() => onOpenWorkspace(row.id)}
              >
                <span className="home-nav-project-label">...</span>
              </button>
            ) : null}
          </div>
        );
      })}
    </>
  );
}
