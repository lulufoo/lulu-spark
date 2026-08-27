import * as api from '../host/api.js';
import { escHtml, repoShortName } from '../shared/utils.js';
import { mountKbReader } from './corpus-viewer.js';
import { setHeaderSyncCorpusContext, clearHeaderSyncCorpusContext } from '../app-shell/header-sync.js';
import { attachCorpusSidebarResize, detachCorpusSidebarResize } from './corpus-sidebar-resize.js';
import { getKbHidePattern, shouldHideEntry } from './corpus-hide-pattern.js';
import { closeFloatingListSelect, createFloatingListSelect } from '../shared/floating-list-select.js';

/**
 * @typedef {{ name: string, relative_path: string, is_dir: boolean, expanded: boolean, loaded: boolean, children: TreeNode[] }} TreeNode
 */

/**
 * @param {string} shortName
 * @param {number | null | undefined} count
 */
export function formatRepoMenuLabel(shortName, count) {
  if (count == null || Number.isNaN(count)) return shortName;
  return `${shortName} · ${Math.floor(count)}`;
}

/**
 * @param {Array<{ full_name: string }>} repos
 * @param {Map<string, number>} [countByRepo]
 */
export function buildRepoPickerOptions(repos, countByRepo = new Map()) {
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

/**
 * @param {Array<{ name?: string, relative_path?: string, is_dir?: boolean }>} entries
 * @param {string} _parentPath
 * @returns {TreeNode[]}
 */
export function buildTreeNodes(entries, _parentPath) {
  const hidePattern = getKbHidePattern();
  const seen = new Set();
  /** @type {TreeNode[]} */
  const nodes = [];
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

/**
 * @param {string} repo
 * @param {string} relativePath
 * @returns {Promise<TreeNode[]>}
 */
export async function loadDirChildren(repo, relativePath) {
  const entries = await api.fetchKbList(repo, relativePath, 'flat');
  return buildTreeNodes(entries, relativePath);
}

export { positionFloatingListMenu } from '../shared/floating-list-menu.js';

/**
 * @param {HTMLElement} container
 * @param {{ repo: string, navigate: (hash: string) => void, initialPath?: string }} opts
 * @returns {() => void}
 */
export function mountCorpusDocList(container, { repo, navigate, initialPath }) {
  let disposed = false;
  /** @type {TreeNode[]} */
  let rootNodes = [];
  let selectedPath = '';
  /** @type {(() => void) | null} */
  let unmountReader = null;
  /** @type {Map<string, Array<{ name: string, relative_path: string, is_dir: boolean }>>} */
  const dirCache = new Map();

  /**
   * @param {TreeNode[]} nodes
   * @param {string} relativePath
   * @returns {TreeNode | null}
   */
  function findNode(nodes, relativePath) {
    for (const node of nodes) {
      if (node.relative_path === relativePath) return node;
      if (node.children.length) {
        const found = findNode(node.children, relativePath);
        if (found) return found;
      }
    }
    return null;
  }

  function renderSidebar() {
    const sidebarEl = container.querySelector('.corpus-doc-sidebar-tree');
    if (!sidebarEl) return;

    /**
     * @param {TreeNode[]} nodes
     * @param {number} depth
     */
    function renderNodes(nodes, depth = 0) {
      return nodes.map((node) => {
        const pad = depth * 16;
        const selectedClass = selectedPath === node.relative_path ? ' selected' : '';
        const childrenHtml = node.expanded && node.children.length
          ? `<div class="corpus-doc-tree-children">${renderNodes(node.children, depth + 1)}</div>`
          : '';
        return `
          <div class="corpus-doc-tree-node" data-relative-path="${escHtml(node.relative_path)}" data-is-dir="${node.is_dir ? '1' : '0'}" style="padding-left:${pad}px">
            <button type="button" class="corpus-doc-tree-label${selectedClass}">${escHtml(node.name)}</button>
          </div>
          ${childrenHtml}
        `;
      }).join('');
    }

    sidebarEl.innerHTML = rootNodes.length
      ? renderNodes(rootNodes)
      : '<div class="corpus-doc-empty">Repository is empty</div>';
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

  /**
   * @param {string} path
   */
  async function mountReaderAt(path) {
    if (unmountReader) {
      unmountReader();
      unmountReader = null;
    }
    const pane = container.querySelector('.corpus-doc-reader-pane');
    if (!pane || !path) return;
    const blobUrl = buildKbBlobUrl(repo, path);
    const { unmount } = await mountKbReader(pane, { repo, path, url: blobUrl });
    if (disposed) {
      unmount();
      return;
    }
    unmountReader = unmount;
  }

  function corpusDocHash(path) {
    return '#/corpus/' + encodeURIComponent(repo) + '?path=' + encodeURIComponent(path);
  }

  /**
   * @param {string} repoFullName
   * @param {string} relativePath
   * @returns {string}
   */
  function buildKbBlobUrl(repoFullName, relativePath) {
    if (!repoFullName || !relativePath) return '';
    const encodedPath = relativePath
      .split('/')
      .map((seg) => encodeURIComponent(seg))
      .join('/');
    return `https://github.com/${repoFullName}/blob/main/${encodedPath}`;
  }

  /** Sync URL for bookmarking without hashchange (avoids router remounting the tree). */
  function syncCorpusHash(relativePath) {
    const hash = corpusDocHash(relativePath);
    if (window.location.hash === hash) return;
    const href = `${window.location.pathname || ''}${window.location.search || ''}${hash}`;
    history.replaceState(null, '', href);
  }

  /** @type {Array<{ full_name: string }>} */
  let sedimentRepos = [];
  /** @type {((next: { value: string, options: ReturnType<typeof buildRepoPickerOptions> }) => void) | null} */
  let repoPickerSync = null;

  async function refreshRepoPickerCounts(repos) {
    if (disposed || !repoPickerSync || !repos.length) return;
    /** @type {Map<string, number>} */
    const countByRepo = new Map();
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

  /**
   * @param {Array<{ full_name: string }>} repos
   */
  function mountRepoPicker(repos) {
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
      onSelect: (fullName) => {
        if (fullName && fullName !== repo) {
          navigate('#/corpus/' + encodeURIComponent(fullName));
        }
      },
    });
    repoPickerSync = sync;
    host.appendChild(picker);
    void refreshRepoPickerCounts(repos);
  }

  /**
   * @param {Array<{ full_name: string }>} repos
   */
  function renderShell(repos) {
    container.innerHTML = `
      <div class="corpus-doc-layout">
        <aside class="corpus-doc-sidebar">
          <div class="corpus-doc-sidebar-header">
            <div class="corpus-repo-picker-host"></div>
          </div>
          <div class="corpus-doc-sidebar-tree"></div>
        </aside>
        <section class="corpus-doc-reader-pane"></section>
      </div>
    `;
    mountRepoPicker(repos);
  }

  const onKbDiffUpdated = (event) => {
    if (event.detail?.repo !== repo) return;
    void reloadFromDisk().catch(() => {});
    if (sedimentRepos.length) void refreshRepoPickerCounts(sedimentRepos);
  };
  window.addEventListener('kb-diff-updated', onKbDiffUpdated);

  const onHidePatternChanged = () => {
    void reloadFromDisk().catch(() => {});
    if (sedimentRepos.length) void refreshRepoPickerCounts(sedimentRepos);
  };
  window.addEventListener('kb:hide-pattern-changed', onHidePatternChanged);

  async function expandNode(node) {
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
          sidebarEl.innerHTML = `<div class="corpus-doc-error">${escHtml(err?.message || 'Failed to load')}</div>`;
        }
        throw err;
      }
    }
    renderSidebar();
  }

  async function ensurePathVisible(relativePath) {
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

  async function onExpandNode(node) {
    if (node.expanded) {
      node.expanded = false;
      renderSidebar();
      return;
    }
    await expandNode(node);
  }

  /**
   * @param {string} path
   */
  async function navigateToPath(path) {
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

  const onSidebarClick = (event) => {
    const labelBtn = event.target.closest('.corpus-doc-tree-label');
    if (!labelBtn) return;
    const nodeEl = labelBtn.closest('.corpus-doc-tree-node');
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

  container.innerHTML = '<div class="corpus-doc-loading">Loading…</div>';

  void (async () => {
    try {
      const reposData = await api.fetchSedimentKbRepos();
      if (disposed) return;
      const repos = reposData.repos || [];

      if (!repo) {
        if (repos.length === 0) {
          container.innerHTML = '<div class="corpus-doc-error">No knowledge libraries yet</div>';
          return;
        }
        navigate('#/corpus/' + encodeURIComponent(repos[0].full_name));
        return;
      }

      const entries = await api.fetchKbList(repo, '', 'flat');
      if (disposed) return;
      dirCache.set('', entries);
      rootNodes = buildTreeNodes(entries, '');
      renderShell(repos);
      attachCorpusSidebarResize(container.querySelector('.corpus-doc-sidebar'));
      setHeaderSyncCorpusContext(repo, reloadFromDisk);
      container.querySelector('.corpus-doc-sidebar')?.addEventListener('click', onSidebarClick);
      renderSidebar();
      if (initialPath) {
        await navigateToPath(initialPath);
      }
    } catch (err) {
      if (disposed) return;
      container.innerHTML = `<div class="corpus-doc-error">${escHtml(err?.message || 'Failed to load')}</div>`;
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
    container.innerHTML = '';
  }

  unmount.navigateToPath = navigateToPath;
  unmount.repo = repo;
  return unmount;
}
