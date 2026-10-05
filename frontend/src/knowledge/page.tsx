import type { ReactNode } from 'react';
import { useLayoutEffect, useRef, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { flushSync } from 'react-dom';
import * as api from '../host/api.ts';
import { mountKbReader } from './viewer.ts';
import { attachKnowledgeSidebarResize, detachKnowledgeSidebarResize } from './ui/sidebar-resize.ts';
import { attachKnowledgeTreeToggle, detachKnowledgeTreeToggle } from './commands/tree-collapse.ts';
import { closeFloatingListSelect, createFloatingListSelect } from '../shared/floating-list-select.ts';
import type { RepoPickerOption, RepoRecord, TreeNode } from './state/types.ts';
import { publishKnowledgeTree } from './state/tree.ts';
import { setKbHidePatternsCache, type KbHidePatternRow } from './state/hide-pattern.ts';
import { buildRepoPickerOptions, buildTreeNodes } from './state/selectors.ts';
import { isKnowledgeMdPath, knowledgeDocHash } from './state/viewer-state.ts';
import {
  applyKbViewerRestore,
  knowledgeLandingHash,
  saveKbViewerState,
} from './commands/viewer-state.ts';
import { commitKbTreeCreate } from './commands/tree-create.ts';
import { commitKbTreeDelete } from './commands/tree-delete.ts';
import { commitKbTreeRename } from './commands/tree-rename.ts';
import {
  creatingPathFor,
  insertDraftNode,
  parentPathForAdd,
  removeTreeNode,
  type CreateKind,
} from './state/tree-entry.ts';
import { openKnowledgeTreeMenu } from './state/tree-menu.ts';
import { resolveRenamePathFromKeydown } from './state/tree-rename.ts';
import { KnowledgeDocLayout } from './ui/sidebar.tsx';

export type { TreeNode } from './state/types.ts';
export { formatRepoMenuLabel, buildRepoPickerOptions, buildTreeNodes } from './state/selectors.ts';
export { positionFloatingListMenu } from '../shared/floating-list-menu.ts';

type KnowledgeDocListHandle = (() => void) & {
  navigateToPath: (path: string) => Promise<void>;
  repo: string;
};

function errorMessage(err: unknown, fallback: string) {
  return err instanceof Error && err.message ? err.message : fallback;
}

function loadingNode() {
  return <div className="knowledge-doc-loading">Loading…</div>;
}

/**
 * Session for one Knowledge repo visit. `renderPage` paints JSX; this function does
 * not create a React root. Production uses KnowledgeDocPage; tests use mountKnowledgeDocList.
 */
function startKnowledgeDocSession(
  container: HTMLElement,
  {
    repo,
    navigate,
    initialPath,
  }: { repo: string; navigate: (hash: string) => void; initialPath?: string },
  renderPage: (node: ReactNode) => void,
) {
  let disposed = false;
  let shelled = false;
  let rootNodes: TreeNode[] = [];
  let selectedPath = initialPath || '';
  let openedPath = '';
  let renamingPath = '';
  let creatingParent = '';
  let creatingKind: CreateKind | '' = '';
  let unmountReader: (() => void) | null = null;
  const dirCache = new Map<string, Array<{ name: string; relative_path: string; is_dir: boolean }>>();
  let sedimentRepos: RepoRecord[] = [];
  let repoPickerSync: ((next: { value: string; options: RepoPickerOption[] }) => void) | null = null;
  let sidebarEl: HTMLElement | null = null;
  let treeEl: HTMLElement | null = null;

  function findNode(nodes: TreeNode[], relativePath: string): TreeNode | null {
    for (const node of nodes) {
      if (node.relative_path === relativePath) return node;
      if (node.children.length) {
        const found = findNode(node.children, relativePath);
        if (found) return found;
      }
    }
    return null;
  }

  function paint(node: ReactNode) {
    if (disposed) return;
    flushSync(() => {
      renderPage(node);
    });
  }

  function renderSidebar() {
    publishKnowledgeTree(rootNodes, selectedPath, '', renamingPath);
  }

  async function reloadFromDisk() {
    dirCache.clear();
    rootNodes = [];
    selectedPath = '';
    openedPath = '';
    renamingPath = '';
    const entries = await api.fetchKbList(repo, '', 'flat');
    if (disposed) return;
    dirCache.set('', entries);
    rootNodes = buildTreeNodes(entries, '');
    renderSidebar();
  }

  async function mountReaderAt(path: string) {
    if (unmountReader) {
      unmountReader();
      unmountReader = null;
    }
    const pane = container.querySelector('.knowledge-doc-reader-pane');
    if (!pane || !path) return;
    const { unmount } = await mountKbReader(pane as HTMLElement, { repo, path });
    if (disposed) {
      unmount();
      return;
    }
    unmountReader = unmount;
    openedPath = path;
  }

  function syncKnowledgeHash(relativePath: string) {
    const hash = knowledgeDocHash(repo, relativePath);
    if (window.location.hash === hash) return;
    const href = `${window.location.pathname || ''}${window.location.search || ''}${hash}`;
    history.replaceState(null, '', href);
  }

  async function refreshRepoPickerCounts(repos: RepoRecord[]) {
    if (disposed || !repoPickerSync || !repos.length) return;
    const countByRepo = new Map<string, number>();
    await Promise.all(
      repos.map(async (r) => {
        const fullName = r.full_name;
        if (!fullName) return;
        try {
          countByRepo.set(fullName, await api.fetchKbDocCount(fullName));
        } catch {
          // keep short label on failure
        }
      }),
    );
    if (disposed || !repoPickerSync) return;
    repoPickerSync({
      value: repo,
      options: buildRepoPickerOptions(repos, countByRepo),
    });
  }

  function mountRepoPicker(repos: RepoRecord[]) {
    sedimentRepos = repos;
    const host = container.querySelector('.knowledge-repo-picker-host');
    if (!host) return;
    host.replaceChildren();
    if (!repos.length) return;

    const { picker, sync } = createFloatingListSelect({
      ariaLabel: 'Select knowledge library',
      pickerClass: 'knowledge-repo-picker',
      menuMaxHeight: 480,
      value: repo,
      options: buildRepoPickerOptions(repos),
      onSelect: (fullName: string) => {
        if (fullName && fullName !== repo) {
          void saveKbViewerState(fullName, '').then(() => {
            if (!disposed) navigate(knowledgeDocHash(fullName));
          });
        }
      },
    });
    repoPickerSync = sync;
    host.appendChild(picker);
    void refreshRepoPickerCounts(repos);
  }

  function renderShell(repos: RepoRecord[]) {
    paint(<KnowledgeDocLayout />);
    attachKnowledgeTreeToggle(container.querySelector('.knowledge-doc-layout'));
    mountRepoPicker(repos);
  }

  const onHidePatternChanged = () => {
    void reloadFromDisk().catch(() => {});
    if (sedimentRepos.length) void refreshRepoPickerCounts(sedimentRepos);
  };
  window.addEventListener('kb:hide-pattern-changed', onHidePatternChanged);

  async function expandNode(node: TreeNode) {
    node.expanded = true;
    if (!node.loaded) {
      try {
        const entries = await api.fetchKbList(repo, node.relative_path, 'flat');
        if (disposed) return;
        node.children = buildTreeNodes(entries, node.relative_path);
        node.loaded = true;
        dirCache.set(node.relative_path, entries);
      } catch (err) {
        node.expanded = false;
        publishKnowledgeTree(rootNodes, selectedPath, errorMessage(err, 'Failed to load'), renamingPath);
        throw err;
      }
    }
    renderSidebar();
  }

  async function ensurePathVisible(relativePath: string) {
    if (!relativePath || !relativePath.includes('/')) return;
    const segments = relativePath.split('/').filter(Boolean);
    segments.pop();
    let built = '';
    for (const seg of segments) {
      built = built ? `${built}/${seg}` : seg;
      const node = findNode(rootNodes, built);
      if (!node?.is_dir || node.expanded) continue;
      await expandNode(node);
    }
  }

  async function onExpandNode(node: TreeNode) {
    if (node.expanded) {
      node.expanded = false;
      renderSidebar();
      return;
    }
    await expandNode(node);
  }

  async function navigateToPath(path: string) {
    const next = path || '';
    if (!shelled) {
      selectedPath = next;
      return;
    }
    if (!next) {
      selectedPath = '';
      openedPath = '';
      if (unmountReader) {
        unmountReader();
        unmountReader = null;
      }
      renderSidebar();
      return;
    }
    if (next === openedPath && unmountReader) {
      renderSidebar();
      return;
    }
    selectedPath = next;
    await ensurePathVisible(next);
    renderSidebar();
    await mountReaderAt(next);
  }

  function clearCreateDraft() {
    if (!creatingKind) return;
    const draftPath = creatingPathFor(creatingParent);
    rootNodes = removeTreeNode(rootNodes, draftPath);
    creatingKind = '';
    creatingParent = '';
    if (renamingPath === draftPath) renamingPath = '';
  }

  function clearRename() {
    if (creatingKind) {
      clearCreateDraft();
      renderSidebar();
      return;
    }
    if (!renamingPath) return;
    renamingPath = '';
    renderSidebar();
  }

  async function startCreate(kind: CreateKind, targetPath: string, isDir: boolean) {
    const parent = parentPathForAdd(targetPath, isDir);
    if (parent) {
      const node = findNode(rootNodes, parent);
      if (node && !node.expanded) await expandNode(node);
    }
    if (disposed) return;
    if (creatingKind) clearCreateDraft();
    rootNodes = insertDraftNode(rootNodes, parent, kind);
    creatingParent = parent;
    creatingKind = kind;
    renamingPath = creatingPathFor(parent);
    selectedPath = renamingPath;
    renderSidebar();
  }

  async function commitCreate(name: string) {
    const parent = creatingParent;
    const kind = creatingKind;
    if (!kind) return;
    try {
      const result = await commitKbTreeCreate(
        { repo, nodes: rootNodes, dirCache },
        parent,
        kind,
        name,
      );
      if (disposed) return;
      if (result.kind === 'invalid') {
        alert(`Create failed: ${result.message}`);
        return;
      }
      rootNodes = result.nodes;
      selectedPath = result.selectedPath;
      creatingKind = '';
      creatingParent = '';
      renamingPath = '';
      renderSidebar();
      if (result.openedPath) {
        await navigateToPath(result.openedPath);
        syncKnowledgeHash(result.openedPath);
        if (isKnowledgeMdPath(result.openedPath)) {
          await saveKbViewerState(repo, result.openedPath);
        }
      }
    } catch (err) {
      alert(`Create failed: ${errorMessage(err, 'unknown error')}`);
    }
  }

  async function commitDelete(path: string) {
    if (!path) return;
    try {
      const result = await commitKbTreeDelete(
        { repo, nodes: rootNodes, dirCache, selectedPath, openedPath },
        path,
      );
      if (disposed) return;
      rootNodes = result.nodes;
      selectedPath = result.selectedPath;
      openedPath = result.openedPath;
      renderSidebar();
      if (result.openedChanged) {
        await navigateToPath(openedPath);
        syncKnowledgeHash(openedPath);
        await saveKbViewerState(repo, isKnowledgeMdPath(openedPath) ? openedPath : '');
      }
    } catch (err) {
      alert(`Delete failed: ${errorMessage(err, 'unknown error')}`);
    }
  }

  async function commitRename(name: string) {
    const from = renamingPath;
    if (!from) return;
    try {
      const result = await commitKbTreeRename(
        {
          repo,
          nodes: rootNodes,
          dirCache,
          selectedPath,
          openedPath,
        },
        from,
        name,
      );
      if (disposed) return;
      if (result.kind === 'invalid') {
        alert(`Rename failed: ${result.message}`);
        clearRename();
        return;
      }
      if (result.kind === 'noop') {
        clearRename();
        return;
      }
      rootNodes = result.nodes;
      selectedPath = result.selectedPath;
      openedPath = result.openedPath;
      renamingPath = '';
      renderSidebar();
      if (result.openedChanged && openedPath) {
        await mountReaderAt(openedPath);
        syncKnowledgeHash(openedPath);
        if (isKnowledgeMdPath(openedPath)) {
          await saveKbViewerState(repo, openedPath);
        }
      }
    } catch (err) {
      alert(`Rename failed: ${errorMessage(err, 'unknown error')}`);
      clearRename();
    }
  }

  const onPageKeyDown = (event: Event) => {
    const ke = event as KeyboardEvent;
    const path = resolveRenamePathFromKeydown({
      key: ke.key,
      repeat: ke.repeat,
      isComposing: ke.isComposing,
      metaKey: ke.metaKey,
      ctrlKey: ke.ctrlKey,
      altKey: ke.altKey,
      target: ke.target,
      host: container,
      renamingPath,
      selectedPath,
    });
    if (!path) return;
    ke.preventDefault();
    ke.stopPropagation();
    selectedPath = path;
    renamingPath = path;
    renderSidebar();
  };

  const onRenameCommit = (event: Event) => {
    const name = String((event as CustomEvent<{ name?: string }>).detail?.name ?? '');
    if (creatingKind) {
      void commitCreate(name);
      return;
    }
    void commitRename(name);
  };

  const onRenameCancel = () => {
    clearRename();
  };

  container.addEventListener('keydown', onPageKeyDown, true);
  window.addEventListener('keydown', onPageKeyDown, true);

  const onSidebarClick = (event: Event) => {
    const target = event.target as Element | null;
    if (target?.closest?.('.knowledge-doc-tree-rename')) return;
    const labelBtn = target?.closest?.('.knowledge-doc-tree-label');
    if (creatingKind) return;
    if (!labelBtn) return;
    const nodeEl = labelBtn.closest('.knowledge-doc-tree-node') as HTMLElement | null;
    const relativePath = nodeEl?.dataset.relativePath ?? '';
    const isDir = nodeEl?.dataset.isDir === '1';
    if (relativePath === creatingPathFor(creatingParent)) return;
    if (isDir) {
      selectedPath = relativePath;
      const node = findNode(rootNodes, relativePath);
      if (!node) return;
      void onExpandNode(node);
      return;
    }
    selectedPath = relativePath;
    void (async () => {
      await navigateToPath(relativePath);
      syncKnowledgeHash(relativePath);
      if (isKnowledgeMdPath(relativePath)) {
        await saveKbViewerState(repo, relativePath);
      }
    })();
  };

  const onTreeContextMenu = (event: MouseEvent) => {
    const target = event.target as Element | null;
    if (target?.closest?.('.knowledge-doc-tree-rename')) return;
    event.preventDefault();
    const nodeEl = target?.closest?.('.knowledge-doc-tree-node') as HTMLElement | null;
    const path = nodeEl?.dataset.relativePath ?? '';
    const isDir = nodeEl ? nodeEl.dataset.isDir === '1' : true;
    if (path && path === creatingPathFor(creatingParent)) return;
    openKnowledgeTreeMenu({ x: event.clientX, y: event.clientY, path, isDir });
  };

  const onTreeAdd = (event: Event) => {
    const detail = (event as CustomEvent<{ kind?: CreateKind; path?: string; isDir?: boolean }>).detail;
    const kind = detail?.kind;
    if (kind !== 'file' && kind !== 'dir') return;
    void startCreate(kind, detail.path ?? '', Boolean(detail.isDir));
  };

  const onTreeDelete = (event: Event) => {
    const path = String((event as CustomEvent<{ path?: string }>).detail?.path ?? '');
    void commitDelete(path);
  };

  paint(loadingNode());

  void (async () => {
    try {
      try {
        const hideData = (await api.fetchKbHidePatterns()) as { patterns?: KbHidePatternRow[] };
        if (disposed) return;
        setKbHidePatternsCache(Array.isArray(hideData?.patterns) ? hideData.patterns : []);
      } catch {
        if (disposed) return;
      }
      const reposData = (await api.fetchSedimentKbRepos()) as { repos?: RepoRecord[] };
      if (disposed) return;
      const repos = reposData.repos || [];

      if (!repo) {
        if (repos.length === 0) {
          paint(<div className="knowledge-doc-error">No knowledge libraries yet</div>);
          return;
        }
        const landingHash = await knowledgeLandingHash(repos.map((item) => item.full_name));
        if (disposed) return;
        if (!landingHash) {
          paint(<div className="knowledge-doc-error">No knowledge libraries yet</div>);
          return;
        }
        navigate(landingHash);
        return;
      }

      const entries = await api.fetchKbList(repo, '', 'flat');
      if (disposed) return;
      dirCache.set('', entries);
      rootNodes = buildTreeNodes(entries, '');
      renderShell(repos);
      attachKnowledgeSidebarResize(container.querySelector('.knowledge-doc-sidebar') as HTMLElement);
      sidebarEl = container.querySelector('.knowledge-doc-sidebar');
      treeEl = container.querySelector('.knowledge-doc-sidebar-tree');
      sidebarEl?.addEventListener('click', onSidebarClick);
      sidebarEl?.addEventListener('kb:tree-rename-commit', onRenameCommit);
      sidebarEl?.addEventListener('kb:tree-rename-cancel', onRenameCancel);
      sidebarEl?.addEventListener('kb:tree-add', onTreeAdd);
      sidebarEl?.addEventListener('kb:tree-delete-confirm', onTreeDelete);
      treeEl?.addEventListener('contextmenu', onTreeContextMenu);
      shelled = true;
      renderSidebar();
      await applyKbViewerRestore({
        repo,
        selectedPath,
        ensureVisible: ensurePathVisible,
        hasFile: (path) => {
          const node = findNode(rootNodes, path);
          return Boolean(node && !node.is_dir);
        },
        openPath: navigateToPath,
        syncHash: syncKnowledgeHash,
      });
    } catch (err) {
      if (disposed) return;
      paint(<div className="knowledge-doc-error">{errorMessage(err, 'Failed to load')}</div>);
    }
  })();

  function dispose() {
    if (disposed) return;
    disposed = true;
    repoPickerSync = null;
    sidebarEl?.removeEventListener('click', onSidebarClick);
    sidebarEl?.removeEventListener('kb:tree-rename-commit', onRenameCommit);
    sidebarEl?.removeEventListener('kb:tree-rename-cancel', onRenameCancel);
    sidebarEl?.removeEventListener('kb:tree-add', onTreeAdd);
    sidebarEl?.removeEventListener('kb:tree-delete-confirm', onTreeDelete);
    treeEl?.removeEventListener('contextmenu', onTreeContextMenu);
    container.removeEventListener('keydown', onPageKeyDown, true);
    window.removeEventListener('keydown', onPageKeyDown, true);
    sidebarEl = null;
    treeEl = null;
    closeFloatingListSelect();
    detachKnowledgeSidebarResize();
    detachKnowledgeTreeToggle();
    unmountReader?.();
    unmountReader = null;
    window.removeEventListener('kb:hide-pattern-changed', onHidePatternChanged);
    publishKnowledgeTree([], '');
  }

  return { dispose, navigateToPath, repo };
}

export type KnowledgeDocPageProps = {
  repo?: string;
  path?: string;
  navigate?: (hash: string) => void;
};

/** Knowledge page. Production child of ShellPages; does not createRoot the page slot. */
export function KnowledgeDocPage({
  repo = '',
  path = '',
  navigate,
}: KnowledgeDocPageProps = {}) {
  const hostRef = useRef<HTMLDivElement | null>(null);
  const sessionRef = useRef<ReturnType<typeof startKnowledgeDocSession> | null>(null);
  const [view, setView] = useState<ReactNode>(loadingNode);

  useLayoutEffect(() => {
    const host = hostRef.current;
    if (!host || typeof navigate !== 'function') return undefined;
    const session = startKnowledgeDocSession(host, { repo, navigate, initialPath: path }, setView);
    sessionRef.current = session;
    return () => {
      session.dispose();
      sessionRef.current = null;
    };
  }, [repo, navigate]);

  useLayoutEffect(() => {
    void sessionRef.current?.navigateToPath(path);
  }, [repo, path]);

  return (
    <div ref={hostRef} className="knowledge-doc-react-host">
      {view}
    </div>
  );
}

/**
 * Test / leftover helper. Production Knowledge is a child of ShellPages, not this root.
 */
export function mountKnowledgeDocList(
  container: HTMLElement,
  {
    repo,
    navigate,
    initialPath,
  }: { repo: string; navigate: (hash: string) => void; initialPath?: string },
): KnowledgeDocListHandle {
  const reactRoot = createRoot(container);
  const session = startKnowledgeDocSession(container, { repo, navigate, initialPath }, (node) => {
    reactRoot.render(node);
  });

  function unmount() {
    session.dispose();
    flushSync(() => {
      reactRoot.unmount();
    });
    container.innerHTML = '';
  }

  const handle = unmount as KnowledgeDocListHandle;
  handle.navigateToPath = session.navigateToPath;
  handle.repo = session.repo;
  return handle;
}
