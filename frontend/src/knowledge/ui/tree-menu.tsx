import { useEffect } from 'react';
import { useSyncExternalStore } from 'react';
import {
  closeKnowledgeTreeMenu,
  knowledgeTreeMenuStore,
  openKnowledgeTreeDelete,
} from '../state/tree-menu.ts';

function emitAdd(kind: 'file' | 'dir') {
  const snap = knowledgeTreeMenuStore.getSnapshot();
  closeKnowledgeTreeMenu();
  document.querySelector('.knowledge-doc-sidebar')?.dispatchEvent(
    new CustomEvent('kb:tree-add', {
      bubbles: true,
      detail: { kind, path: snap.path, isDir: snap.isDir },
    }),
  );
}

export function KnowledgeTreeMenu() {
  const snap = useSyncExternalStore(knowledgeTreeMenuStore.subscribe, knowledgeTreeMenuStore.getSnapshot);

  useEffect(() => {
    if (!snap.open) return undefined;
    const onKey = (event: KeyboardEvent) => {
      if (event.key === 'Escape') closeKnowledgeTreeMenu();
    };
    const onDown = (event: MouseEvent) => {
      const target = event.target as Element | null;
      if (target?.closest('#knowledge-tree-menu')) return;
      closeKnowledgeTreeMenu();
    };
    window.addEventListener('keydown', onKey);
    window.addEventListener('mousedown', onDown);
    return () => {
      window.removeEventListener('keydown', onKey);
      window.removeEventListener('mousedown', onDown);
    };
  }, [snap.open]);

  if (!snap.open) return null;
  return (
    <div
      id="knowledge-tree-menu"
      role="menu"
      style={{ left: snap.x, top: snap.y }}
      onContextMenu={(event) => event.preventDefault()}
    >
      <button type="button" role="menuitem" onClick={() => emitAdd('file')}>
        Add file
      </button>
      <button type="button" role="menuitem" onClick={() => emitAdd('dir')}>
        Add folder
      </button>
      {snap.path ? (
        <button
          type="button"
          role="menuitem"
          onClick={() => {
            const name = snap.path.split('/').pop() || snap.path;
            openKnowledgeTreeDelete({ path: snap.path, name, isDir: snap.isDir });
          }}
        >
          Delete
        </button>
      ) : null}
    </div>
  );
}
