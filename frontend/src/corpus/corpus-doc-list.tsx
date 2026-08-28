import type { ReactNode } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { flushSync } from 'react-dom';
import * as api from '../host/api.ts';
import { repoShortName } from '../shared/utils.ts';
import { mountKbReader } from './corpus-viewer.ts';
import { setHeaderSyncCorpusContext, clearHeaderSyncCorpusContext } from '../app-shell/header-sync.ts';
import { attachCorpusSidebarResize, detachCorpusSidebarResize } from './corpus-sidebar-resize.ts';
import { getKbHidePattern, shouldHideEntry } from './corpus-hide-pattern.ts';
import { closeFloatingListSelect, createFloatingListSelect } from '../shared/floating-list-select.ts';
import { renderToHtml } from '../island.ts';

export type TreeNode = {
  name: string;
  relative_path: string;
  is_dir: boolean;
  expanded: boolean;
  loaded: boolean;
  children: TreeNode[];
};

type RepoPickerOption = { value: string; label: string; title: string };
type RepoRecord = { full_name: string };
type CorpusDocListHandle = (() => void) & {
  navigateToPath: (path: string) => Promise<void>;
  repo: string;
};

export function formatRepoMenuLabel(shortName: string, count: number | null | undefined) {
  if (count == null || Number.isNaN(count)) return shortName;
  return `${shortName} · ${Math.floor(count)}`;
}

export function buildRepoPickerOptions(
  repos: Array<{ full_name: string }>,
  countByRepo: Map<string, number> = new Map(),
): RepoPickerOption[] {
  return repos.map((r) => {
    const fullName = r.full_name || '';
    const short = repoShortName(fullName);
    const count = countByRepo.has(fullName) ? countByRepo.get(fullName) : null;
    return {
      value: fullName,
      label: formatRepoMenuLabel(short, count ?? null),
      title: fullName,
    };
  });
}

export function buildTreeNodes(
  entries: Array<{ name?: string; relative_path?: string; is_dir?: boolean }>,
  _parentPath: string,
): TreeNode[] {
  const hidePattern = getKbHidePattern();
  const seen = new Set<string>();
  const nodes: TreeNode[] = [];
  for (const entry of entries || []) {
    const name = entry.name || (entry.relative_path || '').split('/').pop() || '';
    if (shouldHideEntry(name, hidePattern)) continue;
    const relative_path = entry.relative_path || entry.name || '';
    if (!relative_path || seen.has(relative_path)) continue;
    seen.add(relative_path);
    nodes.push({
      name: entry.name || relative_path.split('/').pop() || relative_path,
      relative_path,
      is_dir: Boolean(entry.is_dir),
      expanded: false,
      loaded: false,
      children: [],
    });
  }
  nodes.sort((a, b) => {
    if (a.is_dir !== b.is_dir) return a.is_dir ? -1 : 1;
    return a.name.localeCompare(b.name);
  });
  return nodes;
}

export { positionFloatingListMenu } from '../shared/floating-list-menu.ts';

function TreeNodes({
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
        className="corpus-doc-tree-node"
        data-relative-path={node.relative_path}
        data-is-dir={node.is_dir ? '1' : '0'}
        style={{ paddingLeft: depth * 16 }}
      >
        <button type="button" className={selected ? 'corpus-doc-tree-label selected' : 'corpus-doc-tree-label'}>
          {node.name}
        </button>
      </div>
      {node.expanded && node.children.length > 0 ? (
        <div className="corpus-doc-tree-children">
          <TreeNodes nodes={node.children} depth={depth + 1} selectedPath={selectedPath} />
        </div>
      ) : null}
    </>
  );
}

function CorpusDocLayout() {
  return (
    <div className="corpus-doc-layout">
      <aside className="corpus-doc-sidebar">
        <div className="corpus-doc-sidebar-header">
          <div className="corpus-repo-picker-host" />
        </div>
        <div className="corpus-doc-sidebar-tree" />
        <div
          className="corpus-sidebar-resizer sidebar-resizer"
          role="separator"
          aria-orientation="vertical"
          aria-label="Resize knowledge tree"
          tabIndex={0}
        />
      </aside>
      <section className="corpus-doc-reader-pane" />
    </div>
  );
}

function errorMessage(err: unknown, fallback: string) {
  return err instanceof Error && err.message ? err.message : fallback;
}

export function mountCorpusDocList(
  container: HTMLElement,
  { repo, navigate, initialPath }: { repo: string; navigate: (hash: string) => void; initialPath?: string },
): CorpusDocListHandle {
  let disposed = false;
  let rootNodes: TreeNode[] = [];
  let selectedPath = '';
  let unmountReader: (() => void) | null = null;
  const dirCache = new Map<string, Array<{ name: string; relative_path: string; is_dir: boolean }>>();
  let root: Root | null = createRoot(container);
  let sedimentRepos: RepoRecord[] = [];
  let repoPickerSync: ((next: { value: string; options: RepoPickerOption[] }) => void) | null = null;

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
    if (!root || disposed) return;
    flushSync(() => {
      root?.render(node);
    });
  }

  function renderSidebar() {
    const sidebarEl = container.querySelector('.corpus-doc-sidebar-tree');
    if (!sidebarEl) return;
    sidebarEl.innerHTML = rootNodes.length
      ? renderToHtml(<TreeNodes nodes={rootNodes} depth={0} selectedPath={selectedPath} />)
      : renderToHtml(<div className="corpus-doc-empty">Repository is empty</div>);
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
        const sidebarEl = container.querySelector('.corpus-doc-sidebar-tree');
        if (sidebarEl) {
          sidebarEl.innerHTML = renderToHtml(
            <div className="corpus-doc-error">{errorMessage(err, 'Failed to load')}</div>,
          );
        }
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
    if (!path) {
      selectedPath = '';
      if (unmountReader) {
        unmountReader();
        unmountReader = null;
      }
      renderSidebar();
      return;
    }
    selectedPath = path;
    await ensurePathVisible(path);
    renderSidebar();
    await mountReaderAt(path);
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

  paint(<div className="corpus-doc-loading">Loading…</div>);

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
      container.querySelector('.corpus-doc-sidebar')?.addEventListener('click', onSidebarClick);
      renderSidebar();
      if (initialPath) {
        await navigateToPath(initialPath);
      }
    } catch (err) {
      if (disposed) return;
      paint(<div className="corpus-doc-error">{errorMessage(err, 'Failed to load')}</div>);
    }
  })();

  function unmount() {
    disposed = true;
    repoPickerSync = null;
    closeFloatingListSelect();
    detachCorpusSidebarResize();
    unmountReader?.();
    unmountReader = null;
    clearHeaderSyncCorpusContext();
    window.removeEventListener('kb-diff-updated', onKbDiffUpdated);
    window.removeEventListener('kb:hide-pattern-changed', onHidePatternChanged);
    if (root) {
      flushSync(() => {
        root?.unmount();
      });
      root = null;
    }
    container.innerHTML = '';
  }

  const handle = unmount as CorpusDocListHandle;
  handle.navigateToPath = navigateToPath;
  handle.repo = repo;
  return handle;
}
