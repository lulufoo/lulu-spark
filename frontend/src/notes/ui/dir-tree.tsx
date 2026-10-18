import { useMemo, useSyncExternalStore } from 'react';
import { getEntryId } from '../../host/state.ts';
import { useHostState } from '../state/host.ts';
import {
  buildDirTree,
  dirKeyOf,
  dirTreeStore,
  findDirNode,
  isDirKey,
  nodeKeyOf,
  toggleDirNodeOpen,
  type DirNode,
} from '../state/dir-tree.ts';
import { selectDate } from '../commands/sidebar.ts';
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

type TreeProps = { depth: number; expanded: string[]; openDocId: string; selectedNode: string };

/** The folders, then the documents, directly inside `node`. */
function NodeContents({ node, depth, expanded, openDocId, selectedNode }: TreeProps & { node: DirNode }) {
  return (
    <>
      {node.children.map((child) => (
        <FolderBlock
          key={child.key}
          node={child}
          depth={depth}
          expanded={expanded}
          openDocId={openDocId}
          selectedNode={selectedNode}
        />
      ))}
      {node.docs.map((doc) => (
        <div
          key={doc.id}
          className="knowledge-doc-tree-node is-file"
          data-id={doc.id}
          data-kind="file"
          style={{ paddingLeft: depth * 16 }}
        >
          <button
            type="button"
            className={`knowledge-doc-tree-label${openDocId === doc.id ? ' selected' : ''}`}
            data-tip={doc.entry.raw_path || doc.name}
            onClick={() => openDoc(doc.id, doc.entry)}
          >
            <Chrome isDir={false} expanded={false} />
            <TreeNodeName name={doc.name} isDir={false} />
          </button>
        </div>
      ))}
    </>
  );
}

/**
 * Clicking a folder lists it on the right (and opens it). Clicking the folder that is already
 * listed and open closes it, so the tree can still be collapsed.
 */
function onFolderClick(node: DirNode, open: boolean, selected: boolean) {
  if (selected && open) toggleDirNodeOpen(node.key);
  else selectDate(dirKeyOf(node.key));
}

function FolderBlock({ node, depth, expanded, openDocId, selectedNode }: TreeProps & { node: DirNode }) {
  const open = expanded.includes(node.key);
  const selected = selectedNode === node.key;
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
          className={`knowledge-doc-tree-label${selected ? ' selected' : ''}`}
          data-tip={node.name}
          aria-expanded={open}
          onClick={() => onFolderClick(node, open, selected)}
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
          <NodeContents
            node={node}
            depth={depth + 1}
            expanded={expanded}
            openDocId={openDocId}
            selectedNode={selectedNode}
          />
        </div>
      ) : null}
    </>
  );
}

/**
 * Directory view of the sidebar: the folders and documents of the project picked in the dropdown,
 * built from `raw_path`. The project itself is not a node. Read-only.
 */
export function DirTree() {
  const host = useHostState();
  const { expanded } = useSyncExternalStore(dirTreeStore.subscribe, dirTreeStore.getSnapshot);
  const project = host.ui.activeTopic;
  const root = useMemo(
    () => (project ? findDirNode(buildDirTree(host.index.data, host.index.topicTitles), project) : null),
    [host.index.data, host.index.topicTitles, project],
  );
  const openDocId =
    host.viewer.entry && host.viewer.outletMode === 'open' ? getEntryId(host.viewer.entry) || '' : '';
  const activeDate = host.ui.activeDate;
  // With a document open the right side is the reader, so no folder reads as the listed one.
  const selectedNode = !openDocId && isDirKey(activeDate) ? nodeKeyOf(activeDate) : '';

  if (!project) return <div className="knowledge-doc-empty">This workspace has no projects yet.</div>;
  if (!root || (root.children.length === 0 && root.docs.length === 0)) {
    return <div className="knowledge-doc-empty">This project has no documents yet.</div>;
  }
  return (
    <div id="sidebar-dir-tree" role="tree">
      <NodeContents node={root} depth={0} expanded={expanded} openDocId={openDocId} selectedNode={selectedNode} />
    </div>
  );
}
