import { state } from '../../frontend/src/host/state.ts';

/** The workspace registry the Host would have loaded: a default workspace and a timeline one. */
export function loadedWorkspaces() {
  return [
    { workspace_id: 'ws-default', workspace_name: 'Default', view_mode: 'directory', is_default: true, icon: '' },
    { workspace_id: 'knowledge', workspace_name: 'Knowledge', view_mode: 'timeline', is_default: false, icon: '' },
  ];
}

/** Put the loaded registry into the shared state; returns the function that puts the old one back. */
export function useLoadedWorkspaces() {
  const before = state.workspaces.list;
  state.workspaces.list = loadedWorkspaces();
  return () => {
    state.workspaces.list = before;
  };
}
