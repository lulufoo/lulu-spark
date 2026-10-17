import { useSyncExternalStore } from 'react';
import { getStateVersion, state, subscribeState } from '../../../host/state.ts';

/** One nav item per enabled workspace; the label follows renames. */
export function WorkspaceNavItems() {
  useSyncExternalStore(subscribeState, getStateVersion, getStateVersion);
  return (
    <>
      {state.workspaces.list.map((workspace) => (
        <button
          key={workspace.id}
          type="button"
          className="settings-nav-item"
          data-panel={`workspace:${workspace.id}`}
        >
          {workspace.name}
        </button>
      ))}
    </>
  );
}
