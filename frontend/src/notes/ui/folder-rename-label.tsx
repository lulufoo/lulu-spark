import { useEffect, useRef, useSyncExternalStore } from 'react';
import { cancelFolderRename, commitFolderRename, setFolderRenameName } from '../commands/folder-rename.ts';
import { folderRenameStore } from '../state/folder-rename.ts';
import { FolderGlyph, TreeChevron } from './dir-tree-glyphs.tsx';

function renameKey() {
  const snap = folderRenameStore.getSnapshot();
  return snap.project ? `${snap.project}/${snap.path}` : '';
}

/** The folder node whose name is being edited, or `''`. */
export function useRenamingFolderKey(): string {
  return useSyncExternalStore(folderRenameStore.subscribe, renameKey);
}

/** The folder's own name, in the row, while Rename is open. */
export function FolderRenameLabel({ nodeKey, open }: { nodeKey: string; open: boolean }) {
  const snap = useSyncExternalStore(folderRenameStore.subscribe, folderRenameStore.getSnapshot);
  const inputRef = useRef<HTMLInputElement>(null);
  const active = renameKey() === nodeKey;

  useEffect(() => {
    if (!active) return;
    inputRef.current?.focus();
    inputRef.current?.select();
  }, [active, snap.tick]);

  if (!active) return null;
  return (
    <div className="knowledge-doc-tree-label">
      <span className="knowledge-doc-tree-twist" aria-hidden="true">
        <TreeChevron expanded={open} />
      </span>
      <span className="knowledge-doc-tree-icon" aria-hidden="true">
        <FolderGlyph />
      </span>
      <input
        ref={inputRef}
        className="folder-create-name"
        aria-label="Folder name"
        value={snap.name}
        disabled={snap.saving}
        onChange={(event) => setFolderRenameName(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === 'Escape') {
            event.preventDefault();
            cancelFolderRename();
            return;
          }
          if (event.key === 'Enter') {
            event.preventDefault();
            void commitFolderRename();
          }
        }}
        onBlur={() => {
          void commitFolderRename();
        }}
      />
    </div>
  );
}
