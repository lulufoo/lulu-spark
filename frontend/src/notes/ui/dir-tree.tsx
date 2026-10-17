import { useMemo, useSyncExternalStore } from 'react';
import { getEntryId } from '../../host/state.ts';
import { selectDirNode } from '../commands/sidebar.ts';
import { useHostState } from '../state/host.ts';
import {
  buildDirTree,
  dirKeyOf,
  dirTreeStore,
  toggleDirNodeOpen,
  type DirNode,
} from '../state/dir-tree.ts';
import type { NoteEntry } from '../state/types.ts';

function openDoc(id: string, entry: NoteEntry) {
  const layer = entry.layers?.includes('raw') ? 'raw' : entry.layers?.[0] || 'raw';
  document.dispatchEvent(new CustomEvent('cta:open-entry', { detail: { id, layer } }));
}

function NodeRow({
  node,
  depth,
  expanded,
  activeKey,
  openDocId,
}: {
  node: DirNode;
  depth: number;
  expanded: string[];
  activeKey: string | null;
  openDocId: string;
}) {
  const open = expanded.includes(node.key);
  const selected = activeKey === dirKeyOf(node.key);
  const expandable = node.children.length > 0 || node.docs.length > 0;

  /** The selected node closes on a second click; any other click selects it and opens its path. */
  function onRowClick() {
    if (selected && open) toggleDirNodeOpen(node.key);
    else selectDirNode(node.key);
  }

  return (
    <li className={`dir-node dir-${node.kind}`} data-node={node.key}>
      <div
        className={`dir-row${selected ? ' active' : ''}`}
        style={{ paddingLeft: `calc(var(--notes-rail) + ${depth * 14}px)` }}
        role="treeitem"
        aria-expanded={expandable ? open : undefined}
        aria-selected={selected}
        onClick={onRowClick}
      >
        <button
          type="button"
          className="dir-chevron"
          aria-label={open ? 'Collapse' : 'Expand'}
          disabled={!expandable}
          onClick={(e) => {
            e.stopPropagation();
            toggleDirNodeOpen(node.key);
          }}
        >
          {expandable ? (open ? '▾' : '▸') : ''}
        </button>
        <span className="dir-name" title={node.key}>{node.name}</span>
        <span className="dir-count">{node.count}</span>
      </div>
      {open ? (
        <ul className="dir-children" role="group">
          {node.children.map((child) => (
            <NodeRow
              key={child.key}
              node={child}
              depth={depth + 1}
              expanded={expanded}
              activeKey={activeKey}
              openDocId={openDocId}
            />
          ))}
          {node.docs.map((doc) => (
            <li key={doc.id} className="dir-doc-item">
              <div
                className={`dir-row dir-doc${openDocId === doc.id ? ' active' : ''}`}
                data-id={doc.id}
                style={{ paddingLeft: `calc(var(--notes-rail) + ${(depth + 1) * 14}px)` }}
                role="treeitem"
                onClick={() => openDoc(doc.id, doc.entry)}
              >
                <span className="dir-chevron" />
                <span className="dir-name" title={doc.entry.raw_path || doc.name}>{doc.name}</span>
              </div>
            </li>
          ))}
        </ul>
      ) : null}
    </li>
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

  if (roots.length === 0) return <div className="dir-empty">No documents yet.</div>;
  return (
    <ul id="sidebar-dir-tree" className="dir-tree" role="tree">
      {roots.map((node) => (
        <NodeRow
          key={node.key}
          node={node}
          depth={0}
          expanded={expanded}
          activeKey={host.ui.activeDate}
          openDocId={openDocId}
        />
      ))}
    </ul>
  );
}
