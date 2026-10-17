import { useSyncExternalStore } from 'react';
import { getStateVersion, state, subscribeState } from '../../../host/state.ts';

/** One nav item per enabled workspace, indented under "Workspaces"; the label follows renames. */
export function WorkspaceNavItems() {
  useSyncExternalStore(subscribeState, getStateVersion, getStateVersion);
  return (
    <>
      {state.workspaces.list.map((workspace) => (
        <button
          key={workspace.id}
          type="button"
          className="settings-nav-item settings-nav-child"
          data-panel={`workspace:${workspace.id}`}
        >
          {workspace.name}
        </button>
      ))}
    </>
  );
}
