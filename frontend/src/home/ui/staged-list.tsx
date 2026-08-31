import type { HubStagedEntry } from '../state/store.ts';

export function StagedList({
  items,
  onOpen,
}: {
  items: HubStagedEntry[];
  onOpen: (path: string, title: string) => void;
}) {
  if (!items.length) return null;
  return (
    <details className="home-chat-staged" data-role="staged-list" open>
      <summary className="home-chat-staged-summary">Staged</summary>
      {items.map((item) => (
        <button
          key={item.id || item.path}
          type="button"
          className="home-chat-staged-item"
          data-role="staged-item"
          data-staged-path={item.path}
          onClick={() => onOpen(item.path, item.title)}
        >
          {item.title}
        </button>
      ))}
    </details>
  );
}
