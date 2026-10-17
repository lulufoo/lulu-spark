import { useSyncExternalStore } from 'react';
import { getStateVersion, state, subscribeState } from '../../../host/state.ts';
import { notesCategoriesStore } from '../../state/settings/notes-categories.ts';
import { NotesCategoriesEditHost } from './notes-categories-edit.tsx';
import { WorkspaceProjectAdd } from './workspace-project-add.tsx';
import { WorkspaceProjectList } from './workspace-project-list.tsx';

/** One page per workspace nav item: its projects, add form, edit dialog and disabled projects. */
export function WorkspaceProjectsPanel() {
  useSyncExternalStore(subscribeState, getStateVersion, getStateVersion);
  const snap = useSyncExternalStore(notesCategoriesStore.subscribe, notesCategoriesStore.getSnapshot);
  const workspace = state.workspaces.list.find((w) => w.id === snap.workspaceId);

  return (
    <div id="settings-panel-workspace" className="settings-panel">
      {workspace ? (
        <div className="notes-workspaces-panel" data-workspace-panel={workspace.id}>
          <h3 id="workspace-panel-title" className="workspace-panel-title">{workspace.name}</h3>
          <p className="notes-cat-lead">
            A project groups documents inside this workspace. Title and description are display only;
            existing documents keep their path.
          </p>
          <WorkspaceProjectList />
          <WorkspaceProjectAdd />
          <NotesCategoriesEditHost />
        </div>
      ) : null}
    </div>
  );
}
