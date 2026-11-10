import { useEffect, useRef, useSyncExternalStore } from 'react';
import { isDirectoryView } from '../commands/workspace.ts';
import { cancelFolderCreate, commitFolderCreate, setFolderCreateName } from '../commands/folder-create.ts';
import { folderCreateStore } from '../state/folder-create.ts';
import { FolderGlyph } from './dir-tree-glyphs.tsx';

/** A folder row under one parent. Enter or blur creates; a blank name cancels. */
export function FolderCreateRow({ nodeKey, depth }: { nodeKey: string; depth: number }) {
  const snap = useSyncExternalStore(folderCreateStore.subscribe, folderCreateStore.getSnapshot);
  const inputRef = useRef<HTMLInputElement>(null);
  const parent = snap.parent === '/' ? '' : snap.parent;
  const target = parent ? `${snap.project}/${parent}` : snap.project;
  const visible = Boolean(snap.project) && isDirectoryView() && nodeKey === target;

  useEffect(() => {
    if (!visible) return;
    inputRef.current?.focus();
  }, [visible, snap.tick]);

  if (!visible) return null;
  return (
    <div
      className="knowledge-doc-tree-node is-dir"
      data-role="folder-create"
      style={{ paddingLeft: depth * 16 }}
    >
      <div className="knowledge-doc-tree-label">
        <span className="knowledge-doc-tree-twist" aria-hidden="true" />
        <span className="knowledge-doc-tree-icon" aria-hidden="true">
          <FolderGlyph />
        </span>
        <input
          ref={inputRef}
          className="folder-create-name"
          aria-label="Folder name"
          placeholder="Folder name"
          value={snap.name}
          disabled={snap.saving}
          onChange={(e) => setFolderCreateName(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Escape') {
              e.preventDefault();
              cancelFolderCreate();
              return;
            }
            if (e.key === 'Enter') {
              e.preventDefault();
              void commitFolderCreate();
            }
          }}
          onBlur={() => {
            void commitFolderCreate();
          }}
        />
      </div>
    </div>
  );
}
