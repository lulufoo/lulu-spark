import * as api from '../api.js';
import { escHtml } from '../utils.js';
import { openKbDoc } from './viewer.js';
import { openKbDiffDialog } from './modals/kb-diff-dialog.js';

const CORPUS_SYNC_LABEL_COMMIT = '↑ 提交变更';
const CORPUS_SYNC_LABEL_PULL = '↓ 更新项目';
const CORPUS_SYNC_LABEL_REFRESH = '⟳ 本地刷新';

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
 * @param {HTMLElement} mainEl
 * @param {Array<{ name: string, relative_path: string, is_dir: boolean }>} entries
 * @param {string} repo
 * @param {(path: string, isDir: boolean) => void} onOpenFile
 */
export function renderMainList(mainEl, entries, repo, onOpenFile) {
  if (!mainEl) return;
  if (!entries || entries.length === 0) {
    mainEl.innerHTML = '<div class="corpus-doc-empty">此目录为空</div>';
    return;
  }
  mainEl.innerHTML = entries.map((entry) => {
    const icon = entry.is_dir ? '📁' : '📄';
    return `<button type="button" class="corpus-doc-main-row" data-relative-path="${escHtml(entry.relative_path)}" data-is-dir="${entry.is_dir ? '1' : '0'}">${icon} ${escHtml(entry.name)}</button>`;
  }).join('');

  mainEl.onclick = (event) => {
    const row = event.target.closest('.corpus-doc-main-row');
    if (!row) return;
    const relativePath = row.dataset.relativePath || '';
    const isDir = row.dataset.isDir === '1';
    if (isDir) {
      onOpenFile(relativePath, true);
    } else {
      void openKbDoc({ repo, path: relativePath });
    }
  };
}

/**
 * @param {HTMLElement} panelEl
 * @param {string} repo
 * @param {{ onLocalRefresh?: () => void | Promise<void> }} [opts]
 */
export function renderCorpusSyncPanel(panelEl, repo, { onLocalRefresh } = {}) {
  if (!panelEl) return;

  panelEl.innerHTML = `
    <div class="corpus-doc-sync-label">SYNC</div>
    <div class="corpus-doc-sync-actions">
      <button type="button" class="corpus-sync-btn corpus-sync-btn-commit" data-action="commit">${CORPUS_SYNC_LABEL_COMMIT}</button>
      <button type="button" class="corpus-sync-btn" data-action="pull">${CORPUS_SYNC_LABEL_PULL}</button>
      <button type="button" class="corpus-sync-btn" data-action="refresh">${CORPUS_SYNC_LABEL_REFRESH}</button>
    </div>
  `;

  const commitBtn = panelEl.querySelector('[data-action="commit"]');
  const pullBtn = panelEl.querySelector('[data-action="pull"]');
  const refreshBtn = panelEl.querySelector('[data-action="refresh"]');

  commitBtn?.addEventListener('click', () => {
    void openKbDiffDialog(repo);
  });

  pullBtn?.addEventListener('click', () => {
    if (!pullBtn) return;
    pullBtn.disabled = true;
    pullBtn.textContent = '更新中…';
    void (async () => {
      try {
        await api.reindexKbRepo(repo);
        for (let i = 0; i < 120; i++) {
          await new Promise((r) => setTimeout(r, 2000));
          const status = await api.getReindexStatus();
          if (status?.status !== 'running') break;
        }
        if (typeof onLocalRefresh === 'function') {
          await onLocalRefresh();
        }
      } catch (err) {
        alert(`更新失败：${err?.message || '未知错误'}`);
      } finally {
        pullBtn.disabled = false;
        pullBtn.textContent = CORPUS_SYNC_LABEL_PULL;
      }
    })();
  });

  refreshBtn?.addEventListener('click', () => {
    if (!refreshBtn || typeof onLocalRefresh !== 'function') return;
    refreshBtn.disabled = true;
    refreshBtn.textContent = '⟳ 刷新中…';
    void Promise.resolve(onLocalRefresh()).finally(() => {
      refreshBtn.disabled = false;
      refreshBtn.textContent = CORPUS_SYNC_LABEL_REFRESH;
    });
  });
}

/**
 * @param {HTMLElement} container
 * @param {{ repo: string, navigate: (hash: string) => void }} opts
 * @returns {() => void}
 */
export function mountCorpusDocList(container, { repo, navigate: _navigate }) {
  let disposed = false;
  /** @type {TreeNode[]} */
  let rootNodes = [];
  let selectedPath = '';
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
        const pad = depth * 12;
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
    await selectDirectory('');
  }

  function renderShell() {
    container.innerHTML = `
      <div class="corpus-doc-layout">
        <aside class="corpus-doc-sidebar">
          <div class="corpus-doc-sidebar-header">${escHtml(repo)}</div>
          <div class="corpus-doc-sync-panel"></div>
          <div class="corpus-doc-sidebar-tree"></div>
        </aside>
        <main class="corpus-doc-main">
          <div class="corpus-doc-main-list"></div>
        </main>
      </div>
    `;
    const syncPanelEl = container.querySelector('.corpus-doc-sync-panel');
    renderCorpusSyncPanel(syncPanelEl, repo, {
      onLocalRefresh: async () => {
        try {
          await reloadFromDisk();
        } catch (err) {
          const mainEl = container.querySelector('.corpus-doc-main-list');
          if (mainEl) {
            mainEl.innerHTML = `<div class="corpus-doc-error">${escHtml(err?.message || '刷新失败')}</div>`;
          }
        }
      },
    });
  }

  const onKbDiffUpdated = (event) => {
    if (event.detail?.repo !== repo) return;
    void reloadFromDisk().catch(() => {});
  };
  window.addEventListener('kb-diff-updated', onKbDiffUpdated);

  async function selectDirectory(relativePath) {
    selectedPath = relativePath;
    renderSidebar();
    const mainEl = container.querySelector('.corpus-doc-main-list');
    if (!mainEl) return;

    let entries = dirCache.get(relativePath);
    if (entries === undefined) {
      try {
        entries = await api.fetchKbList(repo, relativePath, 'flat');
        if (disposed) return;
        dirCache.set(relativePath, entries);
      } catch (err) {
        mainEl.innerHTML = `<div class="corpus-doc-error">${escHtml(err?.message || '加载失败')}</div>`;
        return;
      }
    }

    renderMainList(mainEl, entries, repo, (path, isDir) => {
      if (isDir) {
        void selectDirectory(path);
      }
    });
  }

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
      void selectDirectory(relativePath);
    } else {
      void openKbDoc({ repo, path: relativePath });
    }
  };

  container.innerHTML = '<div class="corpus-doc-loading">加载中…</div>';

  void (async () => {
    try {
      const entries = await api.fetchKbList(repo, '', 'flat');
      if (disposed) return;
      dirCache.set('', entries);
      rootNodes = buildTreeNodes(entries, '');
      renderShell();
      container.querySelector('.corpus-doc-sidebar')?.addEventListener('click', onSidebarClick);
      renderSidebar();
      await selectDirectory('');
    } catch (err) {
      if (disposed) return;
      container.innerHTML = `<div class="corpus-doc-error">${escHtml(err?.message || '加载失败')}</div>`;
    }
  })();

  return () => {
    disposed = true;
    window.removeEventListener('kb-diff-updated', onKbDiffUpdated);
    container.innerHTML = '';
  };
}
