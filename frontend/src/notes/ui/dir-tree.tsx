import { useMemo, useSyncExternalStore } from 'react';
import { getEntryId } from '../../host/state.ts';
import { useHostState } from '../state/host.ts';
import { buildDirTree, dirTreeStore, toggleDirNodeOpen, type DirNode } from '../state/dir-tree.ts';
import type { NoteEntry } from '../state/types.ts';
import { FileGlyph, FolderGlyph, TreeChevron, TreeNodeName } from './dir-tree-glyphs.tsx';

function openDoc(id: string, entry: NoteEntry) {
  const layer = entry.layers?.includes('raw') ? 'raw' : entry.layers?.[0] || 'raw';
  document.dispatchEvent(new CustomEvent('cta:open-entry', { detail: { id, layer } }));
}

function Chrome({ isDir, expanded }: { isDir: boolean; expanded: boolean }) {
  return (
    <>
      <span className="knowledge-doc-tree-twist" aria-hidden="true">
        {isDir ? <TreeChevron expanded={expanded} /> : null}
      </span>
      <span className="knowledge-doc-tree-icon" aria-hidden="true">
        {isDir ? <FolderGlyph /> : <FileGlyph />}
      </span>
    </>
  );
}

function FolderBlock({
  node,
  depth,
  expanded,
  openDocId,
}: {
  node: DirNode;
  depth: number;
  expanded: string[];
  openDocId: string;
}) {
  const open = expanded.includes(node.key);
  return (
    <>
      <div
        className={`knowledge-doc-tree-node is-dir${open ? ' is-expanded' : ''}`}
        data-node={node.key}
        data-kind="dir"
        style={{ paddingLeft: depth * 16 }}
      >
        <button
          type="button"
          className="knowledge-doc-tree-label"
          title={node.name}
          aria-expanded={open}
          onClick={() => toggleDirNodeOpen(node.key)}
        >
          <Chrome isDir expanded={open} />
          <TreeNodeName name={node.name} isDir />
        </button>
      </div>
      {open ? (
        <div
          className="knowledge-doc-tree-children"
          style={{ ['--knowledge-tree-guide' as string]: `${depth * 16 + 12}px` }}
        >
          {node.children.map((child) => (
            <FolderBlock key={child.key} node={child} depth={depth + 1} expanded={expanded} openDocId={openDocId} />
          ))}
          {node.docs.map((doc) => (
            <div
              key={doc.id}
              className="knowledge-doc-tree-node is-file"
              data-id={doc.id}
              data-kind="file"
              style={{ paddingLeft: (depth + 1) * 16 }}
            >
              <button
                type="button"
                className={`knowledge-doc-tree-label${openDocId === doc.id ? ' selected' : ''}`}
                title={doc.entry.raw_path || doc.name}
                onClick={() => openDoc(doc.id, doc.entry)}
              >
                <Chrome isDir={false} expanded={false} />
                <TreeNodeName name={doc.name} isDir={false} />
              </button>
            </div>
          ))}
        </div>
      ) : null}
    </>
  );
}

/** Directory view of the sidebar: project → sub-folders → documents, built from `raw_path`. Read-only. */
export function DirTree() {
  const host = useHostState();
  const { expanded } = useSyncExternalStore(dirTreeStore.subscribe, dirTreeStore.getSnapshot);
  const roots = useMemo(
    () => buildDirTree(host.index.data, host.index.topicTitles),
    [host.index.data, host.index.topicTitles],
  );
  const openDocId =
    host.viewer.entry && host.viewer.outletMode === 'open' ? getEntryId(host.viewer.entry) || '' : '';

  if (roots.length === 0) return <div className="knowledge-doc-empty">No documents yet.</div>;
  return (
    <div id="sidebar-dir-tree" role="tree">
      {roots.map((node) => (
        <FolderBlock key={node.key} node={node} depth={0} expanded={expanded} openDocId={openDocId} />
      ))}
    </div>
  );
}
