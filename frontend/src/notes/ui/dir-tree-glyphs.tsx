/** Glyphs and file-name label of the directory tree; markup and sizes follow main's Knowledge tree. */

export function TreeChevron({ expanded }: { expanded: boolean }) {
  return (
    <svg
      className={expanded ? 'is-expanded' : undefined}
      viewBox="0 0 16 16"
      width="12"
      height="12"
      aria-hidden="true"
      focusable="false"
    >
      <path fill="currentColor" d="M6 4.2a.7.7 0 0 1 1.12-.56l4.1 3.3a.7.7 0 0 1 0 1.12l-4.1 3.3A.7.7 0 0 1 6 10.8V4.2z" />
    </svg>
  );
}

const TREE_STROKE = {
  fill: 'none',
  stroke: 'currentColor',
  strokeWidth: 2,
  strokeLinecap: 'round' as const,
  strokeLinejoin: 'round' as const,
};

export function FolderGlyph() {
  return (
    <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true" focusable="false" {...TREE_STROKE}>
      <path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" />
    </svg>
  );
}

export function FileGlyph() {
  return (
    <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true" focusable="false" {...TREE_STROKE}>
      <path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z" />
      <path d="M14 2v4a2 2 0 0 0 2 2h4" />
    </svg>
  );
}

export function TreeNodeName({ name, isDir }: { name: string; isDir: boolean }) {
  if (isDir) return <span className="knowledge-doc-tree-name">{name}</span>;
  const dot = name.lastIndexOf('.');
  if (dot <= 0 || dot === name.length - 1) return <span className="knowledge-doc-tree-name">{name}</span>;
  return (
    <span className="knowledge-doc-tree-name">
      <span className="knowledge-doc-tree-stem">{name.slice(0, dot)}</span>
      <span className="knowledge-doc-tree-ext">{name.slice(dot)}</span>
    </span>
  );
}
