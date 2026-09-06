import { useLayoutEffect, useRef, useSyncExternalStore } from 'react';
import type { TreeNode } from '../state/types.ts';
import { knowledgeTreeStore } from '../state/tree.ts';
import { ReaderShell } from './viewer/shell.tsx';

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

function FolderGlyph() {
  return (
    <svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true" focusable="false">
      <path
        fill="currentColor"
        d="M1.75 2A1.75 1.75 0 0 0 0 3.75v8.5C0 13.216.784 14 1.75 14h12.5A1.75 1.75 0 0 0 16 12.25v-6.5A1.75 1.75 0 0 0 14.25 4H7.84l-.79-1.05A1.75 1.75 0 0 0 5.66 2H1.75z"
      />
    </svg>
  );
}

function FileGlyph() {
  return (
    <svg viewBox="0 0 16 16" width="16" height="16" aria-hidden="true" focusable="false">
      <path
        fill="currentColor"
        d="M2 1.75C2 .784 2.784 0 3.75 0h6.586c.464 0 .909.184 1.237.513l2.914 2.914c.329.328.513.773.513 1.237v9.586A1.75 1.75 0 0 1 13.25 16h-9.5A1.75 1.75 0 0 1 2 14.25Zm1.75-.25a.25.25 0 0 0-.25.25v12.5c0 .138.112.25.25.25h9.5a.25.25 0 0 0 .25-.25V6h-2.75A1.75 1.75 0 0 1 9 4.25V1.5Zm6.75.062V4.25c0 .138.112.25.25.25h2.688l-.011-.013-2.914-2.914-.013-.011Z"
      />
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
      </aside>
      <section className="knowledge-doc-reader-pane">
        <ReaderShell />
      </section>
    </div>
  );
}
