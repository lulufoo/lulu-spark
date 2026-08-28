import type { ReactNode } from 'react';
import { useLayoutEffect, useRef, useState } from 'react';
import { createRoot } from 'react-dom/client';
import { flushSync } from 'react-dom';
import * as api from '../host/api.ts';
import { mountKbReader } from './viewer.ts';
import { setHeaderSyncCorpusContext, clearHeaderSyncCorpusContext } from '../app-shell/commands/header-sync.ts';
import { attachCorpusSidebarResize, detachCorpusSidebarResize } from './ui/sidebar-resize.ts';
import { closeFloatingListSelect, createFloatingListSelect } from '../shared/floating-list-select.ts';
import type { RepoPickerOption, RepoRecord, TreeNode } from './state/types.ts';
import { publishCorpusTree } from './state/tree.ts';
import { buildRepoPickerOptions, buildTreeNodes } from './state/selectors.ts';
import { CorpusDocLayout } from './ui/sidebar.tsx';

export type { TreeNode } from './state/types.ts';
export { formatRepoMenuLabel, buildRepoPickerOptions, buildTreeNodes } from './state/selectors.ts';
export { positionFloatingListMenu } from '../shared/floating-list-menu.ts';

type CorpusDocListHandle = (() => void) & {
  navigateToPath: (path: string) => Promise<void>;
  repo: string;
};

function errorMessage(err: unknown, fallback: string) {
  return err instanceof Error && err.message ? err.message : fallback;
}

function loadingNode() {
  return <div className="corpus-doc-loading">Loading…</div>;
}

/**
 * Session for one Corpus repo visit. `renderPage` paints JSX; this function does
 * not create a React root. Production uses CorpusDocPage; tests use mountCorpusDocList.
 */
function startCorpusDocSession(
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
    publishCorpusTree(rootNodes, selectedPath);
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
    const pane = container.querySelector('.corpus-doc-reader-pane');
    if (!pane || !path) return;
    const blobUrl = buildKbBlobUrl(repo, path);
    const { unmount } = await mountKbReader(pane as HTMLElement, { repo, path, url: blobUrl });
    if (disposed) {
      unmount();
      return;
    }
    unmountReader = unmount;
  }

  function corpusDocHash(path: string) {
    return `#/corpus/${encodeURIComponent(repo)}?path=${encodeURIComponent(path)}`;
  }

  function buildKbBlobUrl(repoFullName: string, relativePath: string) {
    if (!repoFullName || !relativePath) return '';
    const encodedPath = relativePath
      .split('/')
      .map((seg) => encodeURIComponent(seg))
      .join('/');
    return `https://github.com/${repoFullName}/blob/main/${encodedPath}`;
  }

  function syncCorpusHash(relativePath: string) {
    const hash = corpusDocHash(relativePath);
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
    const host = container.querySelector('.corpus-repo-picker-host');
    if (!host) return;
    host.replaceChildren();
    if (!repos.length) return;

    const { picker, sync } = createFloatingListSelect({
      ariaLabel: 'Select knowledge library',
      pickerClass: 'corpus-repo-picker',
      value: repo,
      options: buildRepoPickerOptions(repos),
      onSelect: (fullName: string) => {
        if (fullName && fullName !== repo) {
          navigate(`#/corpus/${encodeURIComponent(fullName)}`);
        }
      },
    });
    repoPickerSync = sync;
    host.appendChild(picker);
    void refreshRepoPickerCounts(repos);
  }

  function renderShell(repos: RepoRecord[]) {
    paint(<CorpusDocLayout />);
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
        publishCorpusTree(rootNodes, selectedPath, errorMessage(err, 'Failed to load'));
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
    const labelBtn = target?.closest?.('.corpus-doc-tree-label');
    if (!labelBtn) return;
    const nodeEl = labelBtn.closest('.corpus-doc-tree-node') as HTMLElement | null;
    const relativePath = nodeEl?.dataset.relativePath ?? '';
    const isDir = nodeEl?.dataset.isDir === '1';
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
      syncCorpusHash(relativePath);
    })();
  };

  paint(loadingNode());

  void (async () => {
    try {
      const reposData = (await api.fetchSedimentKbRepos()) as { repos?: RepoRecord[] };
      if (disposed) return;
      const repos = reposData.repos || [];

      if (!repo) {
        if (repos.length === 0) {
          paint(<div className="corpus-doc-error">No knowledge libraries yet</div>);
          return;
        }
        navigate(`#/corpus/${encodeURIComponent(repos[0].full_name)}`);
        return;
      }

      const entries = await api.fetchKbList(repo, '', 'flat');
      if (disposed) return;
      dirCache.set('', entries);
      rootNodes = buildTreeNodes(entries, '');
      renderShell(repos);
      attachCorpusSidebarResize(container.querySelector('.corpus-doc-sidebar') as HTMLElement);
      setHeaderSyncCorpusContext(repo, reloadFromDisk);
      sidebarEl = container.querySelector('.corpus-doc-sidebar');
      sidebarEl?.addEventListener('click', onSidebarClick);
      shelled = true;
      renderSidebar();
      if (selectedPath) {
        await navigateToPath(selectedPath);
      }
    } catch (err) {
      if (disposed) return;
      paint(<div className="corpus-doc-error">{errorMessage(err, 'Failed to load')}</div>);
    }
  })();

  function dispose() {
    if (disposed) return;
    disposed = true;
    repoPickerSync = null;
    sidebarEl?.removeEventListener('click', onSidebarClick);
    sidebarEl = null;
    closeFloatingListSelect();
    detachCorpusSidebarResize();
    unmountReader?.();
    unmountReader = null;
    clearHeaderSyncCorpusContext();
    window.removeEventListener('kb-diff-updated', onKbDiffUpdated);
    window.removeEventListener('kb:hide-pattern-changed', onHidePatternChanged);
    publishCorpusTree([], '');
  }

  return { dispose, navigateToPath, repo };
}

export type CorpusDocPageProps = {
  repo?: string;
  path?: string;
  navigate?: (hash: string) => void;
};

/** Corpus page. Production child of ShellPages; does not createRoot the page slot. */
export function CorpusDocPage({
  repo = '',
  path = '',
  navigate,
}: CorpusDocPageProps = {}) {
  const hostRef = useRef<HTMLDivElement | null>(null);
  const sessionRef = useRef<ReturnType<typeof startCorpusDocSession> | null>(null);
  const [view, setView] = useState<ReactNode>(loadingNode);

  useLayoutEffect(() => {
    const host = hostRef.current;
    if (!host || typeof navigate !== 'function') return undefined;
    const session = startCorpusDocSession(host, { repo, navigate, initialPath: path }, setView);
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
    <div ref={hostRef} className="corpus-doc-react-host">
      {view}
    </div>
  );
}

/**
 * Test / leftover helper. Production Corpus is a child of ShellPages, not this root.
 */
export function mountCorpusDocList(
  container: HTMLElement,
  {
    repo,
    navigate,
    initialPath,
  }: { repo: string; navigate: (hash: string) => void; initialPath?: string },
): CorpusDocListHandle {
  const reactRoot = createRoot(container);
  const session = startCorpusDocSession(container, { repo, navigate, initialPath }, (node) => {
    reactRoot.render(node);
  });

  function unmount() {
    session.dispose();
    flushSync(() => {
      reactRoot.unmount();
    });
    container.innerHTML = '';
  }

  const handle = unmount as CorpusDocListHandle;
  handle.navigateToPath = session.navigateToPath;
  handle.repo = session.repo;
  return handle;
}
