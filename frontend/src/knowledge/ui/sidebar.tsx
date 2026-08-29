import { useSyncExternalStore } from 'react';
import type { TreeNode } from '../state/types.ts';
import { knowledgeTreeStore } from '../state/tree.ts';
import { ReaderShell } from './viewer/shell.tsx';

export function TreeNodes({
  nodes,
  depth,
  selectedPath,
}: {
  nodes: TreeNode[];
  depth: number;
  selectedPath: string;
}) {
  return (
    <>
      {nodes.map((node) => (
        <TreeNodeBlock key={node.relative_path} node={node} depth={depth} selectedPath={selectedPath} />
      ))}
    </>
  );
}

function TreeNodeBlock({
  node,
  depth,
  selectedPath,
}: {
  node: TreeNode;
  depth: number;
  selectedPath: string;
}) {
  const selected = selectedPath === node.relative_path;
  return (
    <>
      <div
        className="knowledge-doc-tree-node"
        data-relative-path={node.relative_path}
        data-is-dir={node.is_dir ? '1' : '0'}
        style={{ paddingLeft: depth * 16 }}
      >
        <button type="button" className={selected ? 'knowledge-doc-tree-label selected' : 'knowledge-doc-tree-label'}>
          {node.name}
        </button>
      </div>
      {node.expanded && node.children.length > 0 ? (
        <div className="knowledge-doc-tree-children">
          <TreeNodes nodes={node.children} depth={depth + 1} selectedPath={selectedPath} />
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
  return <TreeNodes nodes={snap.nodes} depth={0} selectedPath={snap.selectedPath} />;
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
