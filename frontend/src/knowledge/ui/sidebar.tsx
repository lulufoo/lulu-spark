import { useLayoutEffect, useRef, useSyncExternalStore } from 'react';
import type { TreeNode } from '../state/types.ts';
import { knowledgeTreeStore } from '../state/tree.ts';
import { KnowledgeTreeDeleteDialog } from './tree-delete-dialog.tsx';
import { KnowledgeTreeMenu } from './tree-menu.tsx';
import { ReaderShell } from './viewer/shell.tsx';
import { PageBackHome } from '../../shared/home-mark.tsx';
import { WindowDragStrip } from '../../shared/window-drag-strip.tsx';

function TreeChevron({ expanded }: { expanded: boolean }) {
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

function FolderGlyph() {
  return (
    <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true" focusable="false" {...TREE_STROKE}>
      <path d="M20 20a2 2 0 0 0 2-2V8a2 2 0 0 0-2-2h-7.9a2 2 0 0 1-1.69-.9L9.6 3.9A2 2 0 0 0 7.93 3H4a2 2 0 0 0-2 2v13a2 2 0 0 0 2 2Z" />
    </svg>
  );
}

function FileGlyph() {
  return (
    <svg viewBox="0 0 24 24" width="16" height="16" aria-hidden="true" focusable="false" {...TREE_STROKE}>
      <path d="M15 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V7Z" />
      <path d="M14 2v4a2 2 0 0 0 2 2h4" />
    </svg>
  );
}

function TreeNodeName({ name, isDir }: { name: string; isDir: boolean }) {
  if (isDir) {
    return <span className="knowledge-doc-tree-name">{name}</span>;
  }
  const dot = name.lastIndexOf('.');
  if (dot <= 0 || dot === name.length - 1) {
    return <span className="knowledge-doc-tree-name">{name}</span>;
  }
  return (
    <span className="knowledge-doc-tree-name">
      <span className="knowledge-doc-tree-stem">{name.slice(0, dot)}</span>
      <span className="knowledge-doc-tree-ext">{name.slice(dot)}</span>
    </span>
  );
}

function KnowledgeRenameInput({ name, isDir }: { name: string; isDir: boolean }) {
  const ref = useRef<HTMLInputElement>(null);
  const done = useRef(false);

  useLayoutEffect(() => {
    const el = ref.current;
    if (!el) return;
    el.focus();
    const dot = name.lastIndexOf('.');
    if (!isDir && dot > 0) {
      el.setSelectionRange(0, dot);
      return;
    }
    el.select();
  }, [isDir, name]);

  function finish(kind: 'commit' | 'cancel') {
    if (done.current) return;
    done.current = true;
    const el = ref.current;
    el?.dispatchEvent(
      new CustomEvent(kind === 'commit' ? 'kb:tree-rename-commit' : 'kb:tree-rename-cancel', {
        bubbles: true,
        detail: { name: el?.value ?? name },
      }),
    );
  }

  return (
    <input
      ref={ref}
      className="knowledge-doc-tree-rename"
      aria-label="Rename"
      defaultValue={name}
      onClick={(event) => event.stopPropagation()}
      onMouseDown={(event) => event.stopPropagation()}
      onKeyDown={(event) => {
        event.stopPropagation();
        if (event.key === 'Enter') {
          event.preventDefault();
          finish('commit');
        } else if (event.key === 'Escape') {
          event.preventDefault();
          finish('cancel');
        }
      }}
      onBlur={() => finish('commit')}
    />
  );
}

export function TreeNodes({
  nodes,
  depth,
  selectedPath,
  renamingPath,
}: {
  nodes: TreeNode[];
  depth: number;
  selectedPath: string;
  renamingPath: string;
}) {
  return (
    <>
      {nodes.map((node) => (
        <TreeNodeBlock
          key={node.relative_path}
          node={node}
          depth={depth}
          selectedPath={selectedPath}
          renamingPath={renamingPath}
        />
      ))}
    </>
  );
}

function TreeNodeBlock({
  node,
  depth,
  selectedPath,
  renamingPath,
}: {
  node: TreeNode;
  depth: number;
  selectedPath: string;
  renamingPath: string;
}) {
  const selected = selectedPath === node.relative_path;
  const renaming = renamingPath === node.relative_path;
  const nodeClass = [
    'knowledge-doc-tree-node',
    node.is_dir ? 'is-dir' : 'is-file',
    node.is_dir && node.expanded ? 'is-expanded' : '',
  ]
    .filter(Boolean)
    .join(' ');
  const labelClass = selected ? 'knowledge-doc-tree-label selected' : 'knowledge-doc-tree-label';
  const chrome = (
    <>
      <span className="knowledge-doc-tree-twist" aria-hidden="true">
        {node.is_dir ? <TreeChevron expanded={node.expanded} /> : null}
      </span>
      <span className="knowledge-doc-tree-icon" aria-hidden="true">
        {node.is_dir ? <FolderGlyph /> : <FileGlyph />}
      </span>
    </>
  );
  return (
    <>
      <div
        className={nodeClass}
        data-relative-path={node.relative_path}
        data-is-dir={node.is_dir ? '1' : '0'}
        data-kind={node.is_dir ? 'dir' : 'file'}
        style={{ paddingLeft: depth * 16 }}
      >
        {renaming ? (
          <div className={labelClass}>
            {chrome}
            <KnowledgeRenameInput name={node.name} isDir={node.is_dir} />
          </div>
        ) : (
          <button
            type="button"
            className={labelClass}
            title={node.name}
            aria-expanded={node.is_dir ? node.expanded : undefined}
          >
            {chrome}
            <TreeNodeName name={node.name} isDir={node.is_dir} />
          </button>
        )}
      </div>
      {node.expanded && node.children.length > 0 ? (
        <div
          className="knowledge-doc-tree-children"
          style={{ ['--knowledge-tree-guide' as string]: `${depth * 16 + 12}px` }}
        >
          <TreeNodes
            nodes={node.children}
            depth={depth + 1}
            selectedPath={selectedPath}
            renamingPath={renamingPath}
          />
        </div>
      ) : null}
    </>
  );
}

export function KnowledgeSidebarTree() {
  const snap = useSyncExternalStore(knowledgeTreeStore.subscribe, knowledgeTreeStore.getSnapshot);
  if (snap.error) {
    return <div className="knowledge-doc-error">{snap.error}</div>;
  }
  if (!snap.nodes.length) {
    return <div className="knowledge-doc-empty">Repository is empty</div>;
  }
  return (
    <TreeNodes
      nodes={snap.nodes}
      depth={0}
      selectedPath={snap.selectedPath}
      renamingPath={snap.renamingPath}
    />
  );
}

export function KnowledgeDocLayout() {
  return (
    <div className="knowledge-doc-layout">
      <aside className="knowledge-doc-sidebar">
        <WindowDragStrip>
          <PageBackHome />
        </WindowDragStrip>
        <div className="knowledge-doc-sidebar-header">
          <div className="knowledge-repo-picker-host" />
        </div>
        <div className="knowledge-doc-sidebar-tree">
          <KnowledgeSidebarTree />
        </div>
        <div
          className="knowledge-sidebar-resizer sidebar-resizer"
          role="separator"
          aria-orientation="vertical"
          aria-label="Resize knowledge tree"
          tabIndex={0}
        />
        <KnowledgeTreeMenu />
        <KnowledgeTreeDeleteDialog />
      </aside>
      <section className="knowledge-doc-reader-pane">
        <ReaderShell />
      </section>
    </div>
  );
}
