import * as api from '../api.js';
import { escHtml } from '../utils.js';

/**
 * @param {Array<{ category_id?: string }>} repos
 * @param {string | null} categoryId
 */
export function filterReposByCategory(repos, categoryId) {
  if (!categoryId) return repos;
  return repos.filter((r) => r.category_id === categoryId);
}

/**
 * @param {HTMLElement} container
 * @param {{ navigate: (hash: string) => void }} opts
 * @returns {() => void}
 */
export function mountCorpusPicker(container, { navigate }) {
  /** @type {Array<{ id: string, name: string }>} */
  let categories = [];
  /** @type {Array<{ full_name: string, category_id?: string }>} */
  let repos = [];
  let selectedCategoryId = '';
  let disposed = false;

  const onCategoryChange = () => {
    selectedCategoryId = container.querySelector('.corpus-picker-category-select')?.value || '';
    renderList();
  };

  const onRepoClick = (event) => {
    const row = event.target.closest('.corpus-picker-repo-row');
    if (!row) return;
    const fullName = row.dataset.fullName;
    if (fullName) navigate('#/corpus/' + encodeURIComponent(fullName));
  };

  function renderList() {
    const listEl = container.querySelector('.corpus-picker-repo-list');
    if (!listEl) return;

    if (repos.length === 0 && categories.length === 0) {
      listEl.innerHTML = '<div class="corpus-picker-empty">暂无沉淀知识库</div>';
      return;
    }

    const filtered = filterReposByCategory(repos, selectedCategoryId || null);
    if (filtered.length === 0) {
      listEl.innerHTML = '<div class="corpus-picker-empty">无匹配仓库</div>';
      return;
    }

    listEl.innerHTML = filtered.map((r) => {
      const fullName = r.full_name || '';
      return `<button type="button" class="corpus-picker-repo-row" data-full-name="${escHtml(fullName)}">${escHtml(fullName)}</button>`;
    }).join('');
  }

  function renderCategoryFilter() {
    const filterEl = container.querySelector('.corpus-picker-filter');
    if (!filterEl) return;

    const options = ['<option value="">全部</option>']
      .concat(categories.map((c) => `<option value="${escHtml(c.id)}">${escHtml(c.name)}</option>`))
      .join('');
    filterEl.innerHTML = `<select class="corpus-picker-category-select">${options}</select>`;
    const select = filterEl.querySelector('.corpus-picker-category-select');
    if (select) {
      select.value = selectedCategoryId;
      select.addEventListener('change', onCategoryChange);
    }
  }

  const onNavClick = (event) => {
    const btn = event.target.closest('[data-nav-target]');
    if (!btn) return;
    const target = btn.dataset.navTarget;
    if (target) navigate(target);
  };

  function renderShell() {
    container.innerHTML = `
      <div class="corpus-picker">
        <div class="corpus-picker-nav">
          <button type="button" class="corpus-nav-back" data-nav-target="#/home">← 返回首页</button>
        </div>
        <div class="corpus-picker-filter"></div>
        <div class="corpus-picker-repo-list"></div>
      </div>
    `;
    container.querySelector('.corpus-picker')?.addEventListener('click', onNavClick);
    container.querySelector('.corpus-picker')?.addEventListener('click', onRepoClick);
    renderCategoryFilter();
    renderList();
  }

  function showError(message) {
    container.innerHTML = `<div class="corpus-picker-error">${escHtml(message || '加载失败')}</div>`;
  }

  container.innerHTML = '<div class="corpus-picker-loading">加载中…</div>';

  void Promise.all([
    api.fetchSedimentKbCategories(),
    api.fetchSedimentKbRepos(),
  ]).then(([catsData, reposData]) => {
    if (disposed) return;
    if (catsData?.error) throw new Error(catsData.error);
    if (reposData?.error) throw new Error(reposData.error);
    categories = catsData.categories || [];
    repos = reposData.repos || [];
    renderShell();
  }).catch((err) => {
    if (disposed) return;
    showError(err?.message || '加载失败');
  });

  return () => {
    disposed = true;
    container.innerHTML = '';
  };
}
