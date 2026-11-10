import { useEffect, useRef, useSyncExternalStore } from 'react';
import { cancelFileRename, commitFileRename, setFileRenameName } from '../commands/file-rename.ts';
import { fileRenameStore } from '../state/file-rename.ts';
import { FileGlyph } from './dir-tree-glyphs.tsx';

function renameId() {
  return fileRenameStore.getSnapshot().id;
}

/** The document whose file name is being edited, or `''`. */
export function useRenamingFileId(): string {
  return useSyncExternalStore(fileRenameStore.subscribe, renameId);
}

/** The file's own name, in the row, while rename is open. */
export function FileRenameLabel({ docId }: { docId: string }) {
  const snap = useSyncExternalStore(fileRenameStore.subscribe, fileRenameStore.getSnapshot);
  const inputRef = useRef<HTMLInputElement>(null);
  const active = snap.id === docId;

  useEffect(() => {
    if (!active) return;
    inputRef.current?.focus();
    inputRef.current?.select();
  }, [active, snap.tick]);

  if (!active) return null;
  return (
    <div className="knowledge-doc-tree-label">
      <span className="knowledge-doc-tree-twist" aria-hidden="true" />
      <span className="knowledge-doc-tree-icon" aria-hidden="true">
        <FileGlyph />
      </span>
      <input
        ref={inputRef}
        className="dir-create-name"
        aria-label="Markdown file name"
        value={snap.name}
        disabled={snap.saving}
        onChange={(event) => setFileRenameName(event.target.value)}
        onKeyDown={(event) => {
          if (event.key === 'Escape') {
            event.preventDefault();
            cancelFileRename();
            return;
          }
          if (event.key === 'Enter') {
            event.preventDefault();
            event.stopPropagation();
            void commitFileRename();
          }
        }}
        onBlur={() => {
          void commitFileRename();
        }}
      />
    </div>
  );
}
