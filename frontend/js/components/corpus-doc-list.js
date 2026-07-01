import * as api from '../api.js';
import { escHtml, repoShortName } from '../utils.js';
import { mountKbReader } from './kb-viewer.js';
import { setHeaderSyncCorpusContext, clearHeaderSyncCorpusContext } from '../header-sync.js';
import { getKbHidePattern, shouldHideEntry } from '../kb-hide-pattern.js';

/**
 * @typedef {{ name: string, relative_path: string, is_dir: boolean, expanded: boolean, loaded: boolean, children: TreeNode[] }} TreeNode
 */

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

/**
 * Position a fixed list menu near a trigger; expand to natural height when space allows.
 * @param {HTMLElement} menu
 * @param {DOMRect} rect
 * @param {number} [viewportHeight]
 * @returns {boolean} true when menu is clipped and caller should scroll selected item into view
 */
export function positionFloatingListMenu(menu, rect, viewportHeight = window.innerHeight) {
  const gap = 4;
  const margin = 8;

  menu.style.left = `${rect.left}px`;
  menu.style.width = `${rect.width}px`;
  menu.style.maxHeight = 'none';
  menu.style.visibility = 'hidden';
  const naturalHeight = menu.scrollHeight;
  menu.style.visibility = '';

  const spaceBelow = viewportHeight - rect.bottom - margin;
  const spaceAbove = rect.top - margin;
  const clippedMaxBelow = Math.floor(spaceBelow * 0.8);
  const clippedMaxAbove = Math.floor(spaceAbove * 0.8);
  let top;
  let maxHeight = null;

  if (naturalHeight <= spaceBelow) {
    top = rect.bottom + gap;
  } else if (naturalHeight <= spaceAbove) {
    top = rect.top - naturalHeight - gap;
  } else if (spaceBelow >= spaceAbove) {
    top = rect.bottom + gap;
    maxHeight = clippedMaxBelow;
  } else {
    maxHeight = clippedMaxAbove;
    top = Math.max(margin, rect.top - maxHeight - gap);
  }

  menu.style.top = `${top}px`;
  if (maxHeight != null) {
    menu.style.maxHeight = `${maxHeight}px`;
  } else {
    menu.style.maxHeight = '';
  }

  return naturalHeight > (maxHeight ?? naturalHeight);
}

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
      : '<div class="corpus-doc-empty">仓库为空</div>';
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
    const { unmount } = await mountKbReader(pane, { repo, path });
    if (disposed) {
      unmount();
      return;
    }
    unmountReader = unmount;
  }

  function corpusDocHash(path) {
    return '#/corpus/' + encodeURIComponent(repo) + '?path=' + encodeURIComponent(path);
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
  /** @type {HTMLElement | null} */
  let repoMenuEl = null;

  function closeRepoMenu() {
    document.querySelectorAll('.corpus-repo-select-trigger[aria-expanded="true"]').forEach((el) => {
      el.setAttribute('aria-expanded', 'false');
      el.closest('.corpus-repo-picker')?.classList.remove('is-open');
    });
    repoMenuEl?.remove();
    repoMenuEl = null;
    document.removeEventListener('mousedown', onDocMouseDownForRepoMenu);
  }

  /** @param {MouseEvent} event */
  function onDocMouseDownForRepoMenu(event) {
    if (event.target.closest('.corpus-repo-select-trigger') || event.target.closest('.corpus-repo-select-menu')) {
      return;
    }
    closeRepoMenu();
  }

  /**
   * @param {HTMLElement} triggerBtn
   */
  function openRepoMenu(triggerBtn) {
    closeRepoMenu();
    if (!sedimentRepos.length) return;

    const rect = triggerBtn.getBoundingClientRect();
    const menu = document.createElement('div');
    menu.className = 'corpus-repo-select-menu';
    menu.setAttribute('role', 'listbox');
    menu.innerHTML = sedimentRepos.map((r) => {
      const fullName = r.full_name || '';
      const label = repoShortName(fullName);
      const selected = fullName === repo ? ' selected' : '';
      return `<button type="button" class="corpus-repo-select-option${selected}" data-full-name="${escHtml(fullName)}" title="${escHtml(fullName)}">${escHtml(label)}</button>`;
    }).join('');

    document.body.appendChild(menu);
    repoMenuEl = menu;

    const clipped = positionFloatingListMenu(menu, rect);

    menu.addEventListener('click', (event) => {
      const option = event.target.closest('.corpus-repo-select-option');
      if (!option) return;
      const fullName = option.dataset.fullName;
      closeRepoMenu();
      if (fullName && fullName !== repo) {
        navigate('#/corpus/' + encodeURIComponent(fullName));
      }
    });

    document.addEventListener('mousedown', onDocMouseDownForRepoMenu);
    triggerBtn.setAttribute('aria-expanded', 'true');
    triggerBtn.closest('.corpus-repo-picker')?.classList.add('is-open');
    if (clipped) {
      const selectedOption = menu.querySelector('.corpus-repo-select-option.selected');
      selectedOption?.scrollIntoView?.({ block: 'nearest' });
    }
  }

  const onRepoPickerClick = (event) => {
    const trigger = event.target.closest('.corpus-repo-select-trigger');
    if (!trigger) return;
    if (repoMenuEl) {
      closeRepoMenu();
      return;
    }
    openRepoMenu(trigger);
  };

  /**
   * @param {Array<{ full_name: string }>} repos
   */
  function renderRepoPickerTrigger(repos) {
    const label = repo ? repoShortName(repo) : '选择知识库';
    return `
      <div class="corpus-repo-picker">
        <button type="button" class="corpus-repo-select-trigger" aria-label="选择知识库" aria-haspopup="listbox" aria-expanded="false">
          <span class="corpus-repo-select-label">${escHtml(label)}</span>
          <span class="corpus-repo-select-chevron" aria-hidden="true"></span>
        </button>
      </div>
    `;
  }

  /**
   * @param {Array<{ full_name: string }>} repos
   */
  function renderShell(repos) {
    sedimentRepos = repos;
    container.innerHTML = `
      <div class="corpus-doc-layout">
        <aside class="corpus-doc-sidebar">
          <div class="corpus-doc-sidebar-header">
            ${renderRepoPickerTrigger(repos)}
          </div>
          <div class="corpus-doc-sidebar-tree"></div>
        </aside>
        <section class="corpus-doc-reader-pane"></section>
      </div>
    `;
  }

  const onKbDiffUpdated = (event) => {
    if (event.detail?.repo !== repo) return;
    void reloadFromDisk().catch(() => {});
  };
  window.addEventListener('kb-diff-updated', onKbDiffUpdated);

  const onHidePatternChanged = () => {
    void reloadFromDisk().catch(() => {});
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
          sidebarEl.innerHTML = `<div class="corpus-doc-error">${escHtml(err?.message || '加载失败')}</div>`;
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

  container.innerHTML = '<div class="corpus-doc-loading">加载中…</div>';

  void (async () => {
    try {
      const reposData = await api.fetchSedimentKbRepos();
      if (disposed) return;
      const repos = reposData.repos || [];

      if (!repo) {
        if (repos.length === 0) {
          container.innerHTML = '<div class="corpus-doc-error">暂无沉淀知识库</div>';
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
      setHeaderSyncCorpusContext(repo, reloadFromDisk);
      container.querySelector('.corpus-repo-picker')?.addEventListener('click', onRepoPickerClick);
      container.querySelector('.corpus-doc-sidebar')?.addEventListener('click', onSidebarClick);
      renderSidebar();
      if (initialPath) {
        await navigateToPath(initialPath);
      }
    } catch (err) {
      if (disposed) return;
      container.innerHTML = `<div class="corpus-doc-error">${escHtml(err?.message || '加载失败')}</div>`;
    }
  })();

  function unmount() {
    disposed = true;
    closeRepoMenu();
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
