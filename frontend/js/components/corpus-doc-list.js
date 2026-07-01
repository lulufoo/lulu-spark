import * as api from '../api.js';
import { escHtml } from '../utils.js';
import { mountKbReader } from './kb-viewer.js';
import { setHeaderSyncCorpusContext, clearHeaderSyncCorpusContext } from '../header-sync.js';

/**
 * @typedef {{ name: string, relative_path: string, is_dir: boolean, expanded: boolean, loaded: boolean, children: TreeNode[] }} TreeNode
 */

/**
 * @param {Array<{ name?: string, relative_path?: string, is_dir?: boolean }>} entries
 * @param {string} _parentPath
 * @returns {TreeNode[]}
 */
export function buildTreeNodes(entries, _parentPath) {
  const seen = new Set();
  /** @type {TreeNode[]} */
  const nodes = [];
  for (const entry of entries || []) {
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
        const expandHtml = node.is_dir
          ? `<button type="button" class="corpus-doc-tree-expand" aria-label="expand">${node.expanded ? '▼' : '▶'}</button>`
          : '<span class="corpus-doc-tree-spacer"></span>';
        const selectedClass = selectedPath === node.relative_path ? ' selected' : '';
        const childrenHtml = node.expanded && node.children.length
          ? `<div class="corpus-doc-tree-children">${renderNodes(node.children, depth + 1)}</div>`
          : '';
        return `
          <div class="corpus-doc-tree-node" data-relative-path="${escHtml(node.relative_path)}" data-is-dir="${node.is_dir ? '1' : '0'}" style="padding-left:${pad}px">
            ${expandHtml}
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

  const onRepoChange = (event) => {
    const select = event.target.closest('.corpus-repo-select');
    if (!select) return;
    const fullName = select.value;
    if (fullName && fullName !== repo) {
      navigate('#/corpus/' + encodeURIComponent(fullName));
    }
  };

  /**
   * @param {Array<{ full_name: string }>} repos
   */
  function renderRepoSelectOptions(repos) {
    return repos.map((r) => {
      const fullName = r.full_name || '';
      const selected = fullName === repo ? ' selected' : '';
      return `<option value="${escHtml(fullName)}"${selected}>${escHtml(fullName)}</option>`;
    }).join('');
  }

  /**
   * @param {Array<{ full_name: string }>} repos
   */
  function renderShell(repos) {
    const repoOptions = renderRepoSelectOptions(repos);
    container.innerHTML = `
      <div class="corpus-doc-layout">
        <aside class="corpus-doc-sidebar">
          <div class="corpus-doc-sidebar-header">
            <select class="corpus-repo-select" aria-label="选择知识库">
              <option value="" disabled${repo ? '' : ' selected'}>选择知识库</option>
              ${repoOptions}
            </select>
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

  async function onExpandNode(node) {
    if (node.expanded) {
      node.expanded = false;
      renderSidebar();
      return;
    }

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
        return;
      }
    }
    renderSidebar();
  }

  const onSidebarClick = (event) => {
    const expandBtn = event.target.closest('.corpus-doc-tree-expand');
    if (expandBtn) {
      const nodeEl = expandBtn.closest('.corpus-doc-tree-node');
      const relativePath = nodeEl?.dataset.relativePath;
      if (!relativePath) return;
      const node = findNode(rootNodes, relativePath);
      if (!node?.is_dir) return;
      void onExpandNode(node);
      return;
    }

    const labelBtn = event.target.closest('.corpus-doc-tree-label');
    if (!labelBtn) return;
    const nodeEl = labelBtn.closest('.corpus-doc-tree-node');
    const relativePath = nodeEl?.dataset.relativePath ?? '';
    const isDir = nodeEl?.dataset.isDir === '1';
    if (isDir) {
      selectedPath = relativePath;
      const node = findNode(rootNodes, relativePath);
      if (!node) return;
      if (!node.expanded) {
        void onExpandNode(node);
      } else {
        renderSidebar();
      }
    } else {
      selectedPath = relativePath;
      navigate(corpusDocHash(relativePath));
    }
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
      container.querySelector('.corpus-repo-select')?.addEventListener('change', onRepoChange);
      container.querySelector('.corpus-doc-sidebar')?.addEventListener('click', onSidebarClick);
      renderSidebar();
      if (initialPath) {
        selectedPath = initialPath;
        renderSidebar();
        await mountReaderAt(initialPath);
      }
    } catch (err) {
      if (disposed) return;
      container.innerHTML = `<div class="corpus-doc-error">${escHtml(err?.message || '加载失败')}</div>`;
    }
  })();

  return () => {
    disposed = true;
    unmountReader?.();
    unmountReader = null;
    clearHeaderSyncCorpusContext();
    window.removeEventListener('kb-diff-updated', onKbDiffUpdated);
    container.innerHTML = '';
  };
}
