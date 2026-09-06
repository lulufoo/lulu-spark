import type { ReactNode } from 'react';
import { useLayoutEffect, useRef, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { flushSync } from 'react-dom';
import * as api from '../host/api.ts';
import { mountKbReader } from './viewer.ts';
import { setHeaderSyncKnowledgeContext, clearHeaderSyncKnowledgeContext } from '../app-shell/commands/header-sync.ts';
import { attachKnowledgeSidebarResize, detachKnowledgeSidebarResize } from './ui/sidebar-resize.ts';
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
  let unmountReader: (() => void) | null = null;
  const dirCache = new Map<string, Array<{ name: string; relative_path: string; is_dir: boolean }>>();
  let sedimentRepos: RepoRecord[] = [];
  let repoPickerSync: ((next: { value: string; options: RepoPickerOption[] }) => void) | null = null;
  let sidebarEl: HTMLElement | null = null;

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
    publishKnowledgeTree(rootNodes, selectedPath);
  }

  async function reloadFromDisk() {
    dirCache.clear();
    rootNodes = [];
    selectedPath = '';
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
    const blobUrl = buildKbBlobUrl(repo, path);
    const { unmount } = await mountKbReader(pane as HTMLElement, { repo, path, url: blobUrl });
    if (disposed) {
      unmount();
      return;
    }
    unmountReader = unmount;
  }

  function buildKbBlobUrl(repoFullName: string, relativePath: string) {
    if (!repoFullName || !relativePath) return '';
    const encodedPath = relativePath
      .split('/')
      .map((seg) => encodeURIComponent(seg))
      .join('/');
    return `https://github.com/${repoFullName}/blob/main/${encodedPath}`;
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
    mountRepoPicker(repos);
  }

  const onKbDiffUpdated = (event: Event) => {
    const detail = (event as CustomEvent<{ repo?: string }>).detail;
    if (detail?.repo !== repo) return;
    void reloadFromDisk().catch(() => {});
    if (sedimentRepos.length) void refreshRepoPickerCounts(sedimentRepos);
  };
  window.addEventListener('kb-diff-updated', onKbDiffUpdated);

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
        publishKnowledgeTree(rootNodes, selectedPath, errorMessage(err, 'Failed to load'));
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
      if (unmountReader) {
        unmountReader();
        unmountReader = null;
      }
      renderSidebar();
      return;
    }
    if (next === selectedPath && unmountReader) {
      renderSidebar();
      return;
    }
    selectedPath = next;
    await ensurePathVisible(next);
    renderSidebar();
    await mountReaderAt(next);
  }

  const onSidebarClick = (event: Event) => {
    const target = event.target as Element | null;
    const labelBtn = target?.closest?.('.knowledge-doc-tree-label');
    if (!labelBtn) return;
    const nodeEl = labelBtn.closest('.knowledge-doc-tree-node') as HTMLElement | null;
    const relativePath = nodeEl?.dataset.relativePath ?? '';
    const isDir = nodeEl?.dataset.isDir === '1';
    if (isDir) {
      selectedPath = relativePath;
      const node = findNode(rootNodes, relativePath);
      if (!node) return;
      void onExpandNode(node);
      return;
    }
    void (async () => {
      await navigateToPath(relativePath);
      syncKnowledgeHash(relativePath);
      if (isKnowledgeMdPath(relativePath)) {
        await saveKbViewerState(repo, relativePath);
      }
    })();
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
      setHeaderSyncKnowledgeContext(repo, reloadFromDisk);
      sidebarEl = container.querySelector('.knowledge-doc-sidebar');
      sidebarEl?.addEventListener('click', onSidebarClick);
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
    sidebarEl = null;
    closeFloatingListSelect();
    detachKnowledgeSidebarResize();
    unmountReader?.();
    unmountReader = null;
    clearHeaderSyncKnowledgeContext();
    window.removeEventListener('kb-diff-updated', onKbDiffUpdated);
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
