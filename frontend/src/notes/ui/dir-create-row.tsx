import { useEffect, useRef, useSyncExternalStore } from 'react';
import { isDirectoryView } from '../commands/workspace.ts';
import { cancelDirCreate, commitDirCreate, setDirCreateName } from '../commands/dir-create.ts';
import { dirCreateStore } from '../state/dir-create.ts';
import { FileGlyph } from './dir-tree-glyphs.tsx';

/** A file row under one folder. Enter or blur creates; a blank name cancels. */
export function DirCreateRow({ nodeKey, depth }: { nodeKey: string; depth: number }) {
  const snap = useSyncExternalStore(dirCreateStore.subscribe, dirCreateStore.getSnapshot);
  const inputRef = useRef<HTMLInputElement>(null);
  const folder = snap.folder === '/' ? '' : snap.folder;
  const target = folder ? `${snap.project}/${folder}` : snap.project;
  const visible = Boolean(snap.project) && isDirectoryView() && nodeKey === target;

  useEffect(() => {
    if (!visible) return;
    inputRef.current?.focus();
  }, [visible, snap.tick]);

  if (!visible) return null;
  return (
    <div
      className="knowledge-doc-tree-node is-file"
      data-role="dir-create"
      style={{ paddingLeft: depth * 16 }}
    >
      <div className="knowledge-doc-tree-label">
        <span className="knowledge-doc-tree-twist" aria-hidden="true" />
        <span className="knowledge-doc-tree-icon" aria-hidden="true">
          <FileGlyph />
        </span>
        <input
          ref={inputRef}
          className="dir-create-name"
          aria-label="Markdown file name"
          placeholder="name.md"
          value={snap.name}
          disabled={snap.saving}
          onChange={(e) => setDirCreateName(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === 'Escape') {
              e.preventDefault();
              cancelDirCreate();
              return;
            }
            if (e.key === 'Enter') {
              e.preventDefault();
              void commitDirCreate();
            }
          }}
          onBlur={() => {
            void commitDirCreate();
          }}
        />
      </div>
    </div>
  );
}
