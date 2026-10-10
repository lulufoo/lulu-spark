import type { HubWorkspaceFile } from '../state/store.ts';

export function WorkspaceList({
  items,
  onOpen,
}: {
  items: HubWorkspaceFile[];
  onOpen: (item: HubWorkspaceFile) => void;
}) {
  if (!items.length) return null;
  return (
    <details className="home-chat-staged" data-role="workspace-list" open>
      <summary className="home-chat-staged-summary">Workspace</summary>
      {items.map((item) => (
        <div key={item.path} className="home-chat-staged-row" data-role="workspace-row">
          <button
            type="button"
            className="home-chat-staged-item"
            data-role="workspace-item"
            data-workspace-path={item.path}
            onClick={() => onOpen(item)}
          >
            <span className="home-chat-staged-title" data-role="workspace-title">
              {item.title}
            </span>
          </button>
        </div>
      ))}
    </details>
  );
}
