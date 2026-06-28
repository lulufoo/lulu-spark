import { state, loadDiffStatus, mergeAnnotations } from './state.js'
import { escHtml } from './utils.js'
import { LAYERS, setGithubUserUrl } from './constants.js'
import * as api from './api.js'
import { buildGroups, renderSidebar, selectDate, applyListFilters, selectTag } from './components/sidebar.js'
import { initSidebarResize } from './components/sidebar-resize.js'
import { enterEditMode, exitEditMode, saveDoc, openCommitDialog, openKbDoc, openDoc } from './components/viewer.js'
import './components/comment-delete.js'
import './components/comments.js'
import './components/kb-viewer.js'
import './components/modals/delete-dialog.js'
import './components/modals/commit-dialog.js'
import { openKbDiffDialog } from './components/modals/kb-diff-dialog.js'
import './components/modals/move-dialog.js'
import { openBase64Dialog } from './components/modals/base64-dialog.js'
import { openQrDialog } from './components/modals/qr-dialog.js'
import { openSettingsDialog } from './components/modals/settings-dialog.js'
import { renderFeed } from './feed.js'
import { initGlobalSearch } from './components/global-search.js'
import { softwareDevSkillsContent } from './skills-software-dev-content.js'
import { normalizeCorpusIndex } from './corpus-index.js'

const titleCache = state.index.titleCache;

// ── Fetch index.json ───────────────────────────────────────────────────────

async function loadIndex({ managedBtn = false } = {}) {
  try {
    const data = await api.fetchIndex();
    state.index.data = normalizeCorpusIndex(data);
    state.index.groupedByDate = buildGroups(state.index.data);
    state.ui.activeTopic = null;
    state.ui.activeTagKey = null;
    applyListFilters();
    renderSidebar();
    await Promise.all([loadDiffStatus(), loadAnnotationsSummary(), loadTagsRegistry()]);
    applyListFilters();
    renderSidebar();
    if (document.body.dataset.knowledgeMode === 'workbench') {
      showWorkbenchKnowledgeShell();
    } else {
      renderKnowledgeHome();
    }
  } catch (e) {
    showError(`无法加载 index.json：${e.message}`);
  }
}

async function loadAnnotationsSummary() {
  try {
    const summary = await api.fetchAnnotationsSummary();
    if (!summary) return;
    state.index.annotations = summary;
    if (state.index.data) mergeAnnotations(state.index.data, summary);
  } catch {
    // Non-fatal: annotations are optional
  }
}

async function loadTagsRegistry() {
  try {
    const data = await api.fetchTagsRegistry();
    if (data?.keys) state.index.tagsRegistry = { keys: data.keys };
  } catch (e) {
    console.error('loadTagsRegistry failed', e);
  }
}

// ── Error display ──────────────────────────────────────────────────────────

function showError(msg) {
  const status = document.getElementById('status');
  status.style.display = '';
  const errDiv = document.createElement('div');
  errDiv.className = 'error-msg';
  errDiv.innerHTML = escHtml(msg) + '<br>';
  const retryBtn = document.createElement('button');
  retryBtn.textContent = '重试';
  retryBtn.addEventListener('click', loadIndex);
  errDiv.appendChild(retryBtn);
  status.innerHTML = '';
  status.appendChild(errDiv);
  document.getElementById('date-heading').style.display = 'none';
  document.getElementById('doc-list').innerHTML = '';
}

function getDefaultArchiveDate() {
  const savedDate = sessionStorage.getItem('cta_active_date');
  if (savedDate && state.index.filteredGroups.find(g => g.date === savedDate)) return savedDate;
  return state.index.filteredGroups.length > 0 ? state.index.filteredGroups[0].date : null;
}

function ensureKnowledgeHome() {
  const main = document.getElementById('main');
  let home = document.getElementById('knowledge-home');
  if (!home) {
    document.getElementById('status').insertAdjacentHTML('beforebegin', '<div id="knowledge-home" class="knowledge-home"></div>');
    home = document.getElementById('knowledge-home');
  }
  return home;
}

function renderKnowledgeHome() {
  document.body.dataset.knowledgeMode = 'home';
  state.ui.activeDate = null;
  const home = ensureKnowledgeHome();
  home.innerHTML = `
    <div class="knowledge-home-title">选择知识库</div>
    <div class="knowledge-home-grid">
      <button type="button" id="knowledge-home-sediment" class="knowledge-home-entry">
        <span class="knowledge-home-entry-title">沉淀知识库</span>
        <span class="knowledge-home-entry-desc">进入沉淀知识库选择视图</span>
      </button>
      <button type="button" id="knowledge-home-workbench" class="knowledge-home-entry">
        <span class="knowledge-home-entry-title">workbench 知识库</span>
        <span class="knowledge-home-entry-desc">继续阅读已有 workbench 文档</span>
      </button>
    </div>`;
  home.style.display = '';
  document.getElementById('status').style.display = 'none';
  document.getElementById('date-heading').style.display = 'none';
  document.getElementById('doc-list').style.display = 'none';
  feedView.style.display = 'none';
  document.getElementById('btn-feed').classList.remove('active');
  bindKnowledgeHomeEvents();
}

function bindKnowledgeHomeEvents() {
  if (document.body.dataset.knowledgeHomeBound === 'true') return;
  document.body.dataset.knowledgeHomeBound = 'true';
  document.addEventListener('click', e => {
    if (e.target.closest('#knowledge-home-sediment')) {
      showSedimentKnowledgeShell();
      return;
    }
    if (e.target.closest('#knowledge-home-workbench')) {
      showWorkbenchKnowledgeShell();
    }
  });
}

function hideKnowledgeHome() {
  const home = document.getElementById('knowledge-home');
  if (home) home.style.display = 'none';
}

function renderSedimentKbHome({ categories = [], repos = [], selectedCategoryId = 'all', selectedRepo = null, error = '' } = {}) {
  const status = document.getElementById('status');
  const visibleRepos = selectedCategoryId === 'all'
    ? repos
    : repos.filter(repo => repo.category_id === selectedCategoryId);
  const selected = selectedRepo;
  const categoryOptions = [
    '<option value="all">全部分类</option>',
    ...categories.map(category => {
      const sel = category.id === selectedCategoryId ? ' selected' : '';
      return `<option value="${escHtml(category.id)}"${sel}>${escHtml(category.name)}</option>`;
    }),
  ].join('');
  const repoHtml = visibleRepos.length > 0
    ? visibleRepos.map(repo => {
      const checked = selected?.full_name === repo.full_name ? ' checked' : '';
      const localBadge = repo.local_exists === true
        ? '<span class="repo-local-badge repo-local-ok">已克隆</span>'
        : '<span class="repo-local-badge repo-local-missing">未克隆</span>';
      const desc = repo.description ? `<div class="repo-list-item-desc">${escHtml(repo.description)}</div>` : '';
      return `<label class="sediment-kb-repo-option">
        <input type="radio" name="sediment-kb-repo" value="${escHtml(repo.full_name)}"${checked} />
        <span>
          <strong>${escHtml(repo.full_name)}</strong>${localBadge}
          <span>${escHtml(repo.category_name || '未分类')}</span>
          ${desc}
        </span>
      </label>`;
    }).join('')
    : '<div class="sediment-kb-empty">暂无仓库候选。可先同步或返回首页。</div>';
  const selectedHtml = selected
    ? `<div class="sediment-kb-selected">已选择：${escHtml(selected.full_name)}</div>`
    : '<div class="sediment-kb-selected">请选择一个具体仓库后继续。</div>';
  const errorHtml = error
    ? `<div class="sediment-kb-error">加载失败：${escHtml(error)}</div>`
    : '';

  status.innerHTML = `<div class="knowledge-shell-placeholder sediment-kb-home">
    <div class="knowledge-home-title">沉淀知识库</div>
    ${errorHtml}
    <label>分类筛选
      <select id="sediment-kb-category-filter">${categoryOptions}</select>
    </label>
    <div class="sediment-kb-repo-list">${repoHtml}</div>
    ${selectedHtml}
    <div class="sediment-kb-actions">
      <button type="button" id="sediment-kb-enter-list"${selected ? '' : ' disabled'}>进入库内列表</button>
      <button type="button" id="sediment-kb-sync">同步</button>
      <button type="button" id="sediment-kb-back">返回</button>
    </div>
  </div>`;

  status.querySelector('#sediment-kb-category-filter').addEventListener('change', e => {
    selectedSedimentKbCategoryId = e.target.value;
    renderSedimentKbHome({
      categories,
      repos,
      selectedCategoryId: selectedSedimentKbCategoryId,
      selectedRepo: selectedSedimentKbRepo,
      error,
    });
  });
  status.querySelectorAll('.sediment-kb-repo-option input').forEach(input => {
    input.addEventListener('change', () => {
      selectSedimentKbRepo(input.value);
      renderSedimentKbHome({
        categories,
        repos,
        selectedCategoryId: selectedSedimentKbCategoryId,
        selectedRepo: selectedSedimentKbRepo,
        error,
      });
    });
  });
  status.querySelector('#sediment-kb-enter-list').addEventListener('click', () => {
    void (async () => {
      const selected = selectedSedimentKbRepo;
      if (!selected) return;
      if (!selected.local_exists) {
        renderSedimentKbHome({
          categories,
          repos,
          selectedCategoryId: selectedSedimentKbCategoryId,
          selectedRepo: selectedSedimentKbRepo,
          error: '该仓库未克隆，不能读取库内文档列表。',
        });
        return;
      }
      if (selected.local_exists) {
        const data = await api.fetchSedimentKbDocs(selected.full_name);
        if (data?.error) throw new Error(data.error);
        renderSedimentKbDocList(data.docs || []);
      }
    })();
  });
  status.querySelector('#sediment-kb-sync').addEventListener('click', () => {
    void showSedimentKnowledgeShell(true);
  });
  status.querySelector('#sediment-kb-back').addEventListener('click', renderKnowledgeHome);
}

function selectSedimentKbRepo(repoFullName) {
  const repo = (_sedimentKbList || []).find(item => item.full_name === repoFullName);
  if (!repo) return;
  selectedSedimentKbRepo = {
    full_name: repo.full_name,
    category_id: repo.category_id,
    category_name: repo.category_name,
    local_exists: repo.local_exists === true,
  };
}

function renderSedimentKbDocList(docs) {
  const status = document.getElementById('status');
  const selectedName = selectedSedimentKbRepo?.full_name || '';
  const rows = Array.isArray(docs) ? docs : [];
  const listHtml = rows.length > 0
    ? rows.map((doc, index) => {
      const label = doc.path || doc.url || '未知文档';
      if (!doc.repo || !doc.path) {
        return `<div class="sediment-kb-doc-row sediment-kb-doc-invalid">
          <button type="button" disabled>缺少 repo 或 path</button>
          <span>${escHtml(label)}</span>
        </div>`;
      }
      return `<button type="button" class="sediment-kb-doc-row sediment-kb-doc-item" data-index="${index}">
        <span>${escHtml(doc.path)}</span>
      </button>`;
    }).join('')
    : '<div class="sediment-kb-doc-empty">该知识库暂无文档。</div>';

  status.innerHTML = `<div class="knowledge-shell-placeholder sediment-kb-doc-list">
    <div class="knowledge-home-title">${escHtml(selectedName)} 文档列表</div>
    <div class="sediment-kb-doc-list-body">${listHtml}</div>
    <div class="sediment-kb-actions">
      <button type="button" id="sediment-kb-doc-back">返回仓库选择</button>
    </div>
  </div>`;

  status.querySelectorAll('.sediment-kb-doc-item').forEach(btn => {
    btn.addEventListener('click', () => {
      void openSedimentKbDoc(rows[Number(btn.dataset.index)]);
    });
  });
  status.querySelector('#sediment-kb-doc-back').addEventListener('click', () => {
    void showSedimentKnowledgeShell(false);
  });
}

async function openSedimentKbDoc(doc) {
  if (!doc?.repo || !doc?.path) return;
  await openKbDoc({ repo: doc.repo, path: doc.path, url: doc.url });
}

async function showSedimentKnowledgeShell(forceRefresh = false) {
  document.body.dataset.knowledgeMode = 'sediment';
  hideKnowledgeHome();
  feedView.style.display = 'none';
  document.getElementById('btn-feed').classList.remove('active');
  document.getElementById('date-heading').style.display = 'none';
  document.getElementById('doc-list').style.display = 'none';
  const status = document.getElementById('status');
  status.style.display = '';
  status.innerHTML = '<div class="knowledge-shell-placeholder">加载沉淀知识库…</div>';
  try {
    if (forceRefresh || !_sedimentKbList || !_sedimentKbCategories || _sedimentKbError) {
      _sedimentKbError = null;
      const [catsData, reposData] = await Promise.all([
        api.fetchSedimentKbCategories(),
        api.fetchSedimentKbRepos(),
      ]);
      _sedimentKbCategories = catsData.categories || [];
      _sedimentKbList = (reposData.repos || []).map(repo => ({
        full_name: repo.full_name,
        name: (repo.full_name || '').split('/').pop() || repo.full_name,
        description: repo.description || '',
        category_id: repo.category_id,
        category_name: repo.category_name,
        local_exists: repo.local_exists === true,
      }));
    }
    renderSedimentKbHome({
      categories: _sedimentKbCategories,
      repos: _sedimentKbList,
      selectedCategoryId: selectedSedimentKbCategoryId,
      selectedRepo: selectedSedimentKbRepo,
    });
  } catch (e) {
    _sedimentKbError = e.message || String(e);
    _sedimentKbList = [];
    renderSedimentKbHome({
      categories: _sedimentKbCategories || [],
      repos: [],
      selectedCategoryId: selectedSedimentKbCategoryId,
      selectedRepo: null,
      error: _sedimentKbError,
    });
  }
}

function showWorkbenchKnowledgeShell() {
  document.body.dataset.knowledgeMode = 'workbench';
  hideKnowledgeHome();
  showArchiveView();
  const targetDate = getDefaultArchiveDate();
  if (targetDate) selectDate(targetDate);
}

// ── Pull project ───────────────────────────────────────────────────────────

async function pullProject() {
  const btn = document.getElementById('btn-pull');
  btn.disabled = true;
  btn.textContent = '更新中…';
  try {
    const data = await api.pullProject();
    if (data.error) throw new Error((data.error || '') + (data.stderr ? '\n' + data.stderr : ''));

    titleCache.clear();
    if (state.index.data) {
      for (const entry of Object.values(state.index.data)) {
        LAYERS.forEach(l => delete entry[`_unreachable_${l}`]);
      }
    }
    await loadIndex();
  } catch (e) {
    alert(`更新失败：${e.message}`);
  } finally {
    btn.disabled = false;
    btn.textContent = '↓ 更新项目';
  }
}

// ── Repo menu helpers ──────────────────────────────────────────────────────

async function _reloadTopicsIntoState() {
  try {
    const data = await api.fetchTopics();
    const descMap = {};
    const repoMap = {};
    for (const t of (data.topics || [])) {
      if (!t.repo) continue;
      const key = t.dir || t.repo.split('/')[1];
      if (t.description) descMap[key] = t.description;
      repoMap[key] = `https://github.com/${t.repo}`;
    }
    state.index.topicDescriptions = descMap;
    state.index.topicRepos = repoMap;
  } catch (e) {
    console.warn('fetchTopics failed:', e.message);
  }
}

// ── Repo menu dropdown ─────────────────────────────────────────────────────

const _repoMenuWrap = document.getElementById('repo-menu-wrap');
const _repoMenuDropdown = document.getElementById('repo-menu-dropdown');
const _syncMenuDropdown = document.getElementById('sync-menu-dropdown');
const _toolsMenuDropdown = document.getElementById('tools-menu-dropdown');
const _skillsMenuDropdown = document.getElementById('skills-menu-dropdown');

function _closeAllMenuDropdowns() {
  _repoMenuDropdown.classList.remove('open');
  _syncMenuDropdown.classList.remove('open');
  _toolsMenuDropdown.classList.remove('open');
  _skillsMenuDropdown.classList.remove('open');
}

// ── 沉淀知识库（sediment-kb：精选列表，按分类分组）────────────────────────

function _closeSedimentKbListDialog() {
  document.getElementById('repo-list-dialog').classList.remove('open');
}

function _closeSedimentKbAddDialog() {
  document.getElementById('sediment-kb-add-dialog').classList.remove('open');
}

function _closeSedimentKbManageDialog() {
  document.getElementById('sediment-kb-manage-dialog').classList.remove('open');
}

let _kbCorpusStatus = null;
let _kbCorpusDiffStatus = null;
let _sedimentKbList = null;
let _sedimentKbError = null;
let _sedimentKbCategories = null;
let selectedSedimentKbCategoryId = 'all';
let selectedSedimentKbRepo = null;

function _setSedimentKbError(elId, message) {
  const el = document.getElementById(elId);
  if (!el) return;
  if (message) {
    el.textContent = message;
    el.style.display = '';
  } else {
    el.textContent = '';
    el.style.display = 'none';
  }
}

async function _ensureSedimentKbCategories() {
  if (_sedimentKbCategories) return _sedimentKbCategories;
  const data = await api.fetchSedimentKbCategories();
  _sedimentKbCategories = data.categories || [];
  return _sedimentKbCategories;
}

function renderSedimentKbListByCategory(repos) {
  const content = document.getElementById('repo-list-content');
  if (!repos || repos.length === 0) {
    content.innerHTML = '<div id="repo-list-loading">未找到任何仓库</div>';
    return;
  }

  const categories = _sedimentKbCategories || [];
  const statusMap = {};
  if (_kbCorpusStatus) {
    for (const s of _kbCorpusStatus) statusMap[s.full_name] = s;
  }

  const groups = {};
  for (const r of repos) {
    const key = r.category_name || '未分类';
    if (!groups[key]) groups[key] = [];
    groups[key].push(r);
  }
  for (const key of Object.keys(groups)) {
    groups[key].sort((a, b) => (a.name || '').localeCompare(b.name || '', 'en', { sensitivity: 'base' }));
  }

  const sortedKeys = Object.keys(groups).sort((a, b) => {
    if (a === '未分类') return 1;
    if (b === '未分类') return -1;
    return a.localeCompare(b, 'en', { sensitivity: 'base' });
  });

  const html = sortedKeys.map(key => {
    const items = groups[key].map(r => {
      const name = escHtml(r.name || r.full_name || '');
      const desc = r.description ? `<div class="repo-list-item-desc">${escHtml(r.description)}</div>` : '';
      const url = `https://github.com/${escHtml(r.full_name || r.name)}`;

      let localBadge = '';
      let diffBtnHtml = '';
      let syncBtnHtml = '';
      const st = statusMap[r.full_name];
      if (st) {
        localBadge = st.local_exists
          ? `<span class="repo-local-badge repo-local-ok">已克隆</span>`
          : `<span class="repo-local-badge repo-local-missing">未克隆</span>`;
        if (_kbCorpusDiffStatus?.get(r.full_name) === true) {
          diffBtnHtml = `<button class="repo-diff-badge" data-repo="${escHtml(r.full_name)}" title="查看本地变更">✎</button>`;
        }
        syncBtnHtml = `<button class="repo-sync-btn" data-repo="${escHtml(r.full_name)}">SYNC</button>`;
      }

      const catSelectOptions = categories.map(c => {
        const sel = c.id === r.category_id ? ' selected' : '';
        return `<option value="${escHtml(c.id)}"${sel}>${escHtml(c.name)}</option>`;
      }).join('');

      return `<div class="repo-list-item">
        <div class="repo-list-item-info">
          <div class="repo-list-item-name">${name}${localBadge}</div>
          ${desc}
        </div>
        <div class="repo-list-item-actions">
          <select class="sediment-kb-inline-category" data-repo="${escHtml(r.full_name)}">${catSelectOptions}</select>
          ${diffBtnHtml}
          ${syncBtnHtml}
          <a class="repo-list-item-link" href="${url}" target="_blank" rel="noopener noreferrer">Link ↗</a>
          <button type="button" class="sediment-kb-delete-btn" data-repo="${escHtml(r.full_name)}" title="从精选列表移除">删除</button>
        </div>
      </div>`;
    }).join('');
    return `<div class="repo-list-group-title">${escHtml(key)}</div>${items}`;
  }).join('');

  content.innerHTML = html;

  content.querySelectorAll('.repo-sync-btn').forEach(btn => {
    btn.addEventListener('click', async () => {
      const repo = btn.dataset.repo;
      btn.disabled = true;
      btn.textContent = '…';
      try {
        await api.reindexKbRepo(repo);
        const poll = () => api.getReindexStatus();
        for (let i = 0; i < 120; i++) {
          await new Promise(r => setTimeout(r, 2000));
          const s = await poll();
          if (s?.status !== 'running') break;
        }
        _kbCorpusStatus = null;
        await loadSedimentKbList(true);
      } catch (e) {
        alert(`同步失败：${e.message}`);
        btn.disabled = false;
        btn.textContent = 'SYNC';
      }
    });
  });

  content.querySelectorAll('.repo-diff-badge').forEach(btn => {
    btn.addEventListener('click', () => {
      openKbDiffDialog(btn.dataset.repo);
    });
  });

  content.querySelectorAll('.sediment-kb-inline-category').forEach(sel => {
    sel.addEventListener('change', () => {
      onInlineCategoryChange(sel.dataset.repo, sel.value);
    });
  });

  content.querySelectorAll('.sediment-kb-delete-btn').forEach(btn => {
    btn.addEventListener('click', () => {
      onDeleteSedimentKbRepo(btn.dataset.repo);
    });
  });
}

async function onInlineCategoryChange(fullName, categoryId) {
  try {
    const res = await api.updateSedimentKbRepoCategory(fullName, categoryId);
    if (res?.error) throw new Error(res.error);
    _sedimentKbList = null;
    await loadSedimentKbList(true);
  } catch (e) {
    alert(`更新分类失败：${e.message}`);
    await loadSedimentKbList(true);
  }
}

async function onDeleteSedimentKbRepo(fullName) {
  try {
    const res = await api.removeSedimentKbRepo(fullName);
    if (res?.error) throw new Error(res.error);
    _sedimentKbList = null;
    await loadSedimentKbList(true);
  } catch (e) {
    alert(`删除失败：${e.message}`);
  }
}

async function loadSedimentKbList(forceRefresh = false) {
  if (!forceRefresh && _sedimentKbList && !_sedimentKbError) {
    renderSedimentKbListByCategory(_sedimentKbList);
    return;
  }
  _kbCorpusStatus = null;
  _sedimentKbList = null;
  _sedimentKbError = null;
  _kbCorpusDiffStatus = null;
  const content = document.getElementById('repo-list-content');
  try {
    const [reposData, catsData] = await Promise.all([
      api.fetchSedimentKbRepos(),
      api.fetchSedimentKbCategories(),
    ]);
    if (reposData?.error) throw new Error(reposData.error);
    _sedimentKbCategories = catsData.categories || [];
    _sedimentKbList = (reposData.repos || []).map((r) => ({
      full_name: r.full_name,
      name: (r.full_name || '').split('/').pop() || r.full_name,
      description: r.description || '',
      category_id: r.category_id,
      category_name: r.category_name,
      local_exists: r.local_exists === true,
    }));
    _kbCorpusStatus = _sedimentKbList.map((r) => ({
      full_name: r.full_name,
      name: r.name,
      description: r.description,
      local_exists: r.local_exists === true,
    }));
  } catch (e) {
    _sedimentKbError = e.message || String(e);
    _sedimentKbList = [];
    _kbCorpusStatus = [];
    if (content) {
      content.innerHTML = `<div id="repo-list-loading" style="color:#cf222e">加载失败：${escHtml(_sedimentKbError)}</div>`;
    }
    return;
  }
  renderSedimentKbListByCategory(_sedimentKbList);

  _kbCorpusDiffStatus = null;
  try {
    const diffData = await api.fetchKbDiffStatus();
    _kbCorpusDiffStatus = new Map(
      (diffData?.repos || []).map((repo) => [repo.full_name, repo.has_changes === true]),
    );
  } catch {
    _kbCorpusDiffStatus = new Map();
  }
  renderSedimentKbListByCategory(_sedimentKbList);
}

async function openSedimentKbListDialog() {
  const title = document.querySelector('#repo-list-title-group h3');
  if (title) title.textContent = '☰ 沉淀知识库列表';
  document.getElementById('repo-list-dialog').classList.add('open');
  const content = document.getElementById('repo-list-content');
  content.innerHTML = '<div id="repo-list-loading">加载中…</div>';
  await loadSedimentKbList(true);
}

async function openSedimentKbAddDialog() {
  const urlInput = document.getElementById('sediment-kb-add-url');
  _setSedimentKbError('sediment-kb-add-error', '');
  urlInput.value = '';
  document.getElementById('sediment-kb-add-description').value = '';
  try {
    const categories = await _ensureSedimentKbCategories();
    const catSelect = document.getElementById('sediment-kb-add-category');
    catSelect.innerHTML = categories.map(c =>
      `<option value="${escHtml(c.id)}">${escHtml(c.name)}</option>`,
    ).join('');
  } catch (e) {
    document.getElementById('sediment-kb-add-category').innerHTML =
      '<option value="uncategorized">未分类</option>';
  }
  document.getElementById('sediment-kb-add-dialog').classList.add('open');
  urlInput.focus();
}

function _renderSedimentKbManageList(categories) {
  const list = document.getElementById('sediment-kb-manage-list');
  list.innerHTML = categories.map(c => {
    const isProtected = c.id === 'uncategorized';
    const deleteBtn = isProtected
      ? ''
      : `<button type="button" class="sediment-kb-cat-delete-btn" data-id="${escHtml(c.id)}">删除</button>`;
    const nameCell = isProtected
      ? `<span class="sediment-kb-cat-name-readonly">${escHtml(c.name)}</span>`
      : `<input class="sediment-kb-cat-rename-input" data-id="${escHtml(c.id)}" type="text" value="${escHtml(c.name)}" />`;
    return `<div class="sediment-kb-manage-row">${nameCell}${deleteBtn}</div>`;
  }).join('');

  list.querySelectorAll('.sediment-kb-cat-rename-input').forEach(input => {
    input.addEventListener('change', async () => {
      const id = input.dataset.id;
      const name = input.value.trim();
      if (!name) return;
      try {
        const res = await api.renameSedimentKbCategory(id, name);
        if (res?.error) throw new Error(res.error);
        _sedimentKbCategories = null;
        const data = await api.fetchSedimentKbCategories();
        _sedimentKbCategories = data.categories || [];
        _renderSedimentKbManageList(_sedimentKbCategories);
      } catch (e) {
        _setSedimentKbError('sediment-kb-manage-error', e.message);
      }
    });
  });

  list.querySelectorAll('.sediment-kb-cat-delete-btn').forEach(btn => {
    btn.addEventListener('click', async () => {
      try {
        const res = await api.removeSedimentKbCategory(btn.dataset.id);
        if (res?.error) throw new Error(res.error);
        _sedimentKbCategories = null;
        _sedimentKbList = null;
        const data = await api.fetchSedimentKbCategories();
        _sedimentKbCategories = data.categories || [];
        _renderSedimentKbManageList(_sedimentKbCategories);
        _setSedimentKbError('sediment-kb-manage-error', '');
      } catch (e) {
        _setSedimentKbError('sediment-kb-manage-error', e.message);
      }
    });
  });
}

async function openSedimentKbManageDialog() {
  _setSedimentKbError('sediment-kb-manage-error', '');
  document.getElementById('sediment-kb-manage-new-name').value = '';
  try {
    const categories = await _ensureSedimentKbCategories();
    _renderSedimentKbManageList(categories);
  } catch (e) {
    document.getElementById('sediment-kb-manage-list').innerHTML =
      `<div class="sediment-kb-error">${escHtml(e.message)}</div>`;
  }
  document.getElementById('sediment-kb-manage-dialog').classList.add('open');
}

window.addEventListener('kb-diff-updated', () => {
  void loadSedimentKbList(true);
});

document.getElementById('btn-sediment-kb-list').addEventListener('click', () => {
  _repoMenuDropdown.classList.remove('open');
  void openSedimentKbListDialog();
});

document.getElementById('btn-sediment-kb-add').addEventListener('click', () => {
  _repoMenuDropdown.classList.remove('open');
  void openSedimentKbAddDialog();
});

document.getElementById('btn-sediment-kb-manage').addEventListener('click', () => {
  _repoMenuDropdown.classList.remove('open');
  void openSedimentKbManageDialog();
});

document.getElementById('btn-repo-list-refresh').addEventListener('click', () => {
  void loadSedimentKbList(true);
});

document.getElementById('btn-repo-list-close').addEventListener('click', _closeSedimentKbListDialog);
document.getElementById('repo-list-dialog').addEventListener('click', e => {
  if (e.target === document.getElementById('repo-list-dialog')) _closeSedimentKbListDialog();
});

document.getElementById('btn-sediment-kb-add-cancel').addEventListener('click', _closeSedimentKbAddDialog);
document.getElementById('sediment-kb-add-dialog').addEventListener('click', e => {
  if (e.target === document.getElementById('sediment-kb-add-dialog')) _closeSedimentKbAddDialog();
});
document.getElementById('btn-sediment-kb-add-submit').addEventListener('click', () => {
  void (async () => {
    const urlInput = document.getElementById('sediment-kb-add-url');
    const catSelect = document.getElementById('sediment-kb-add-category');
    const descInput = document.getElementById('sediment-kb-add-description');
    const submitBtn = document.getElementById('btn-sediment-kb-add-submit');
    const fullName = urlInput.value.trim();
    if (!fullName) {
      _setSedimentKbError('sediment-kb-add-error', '请输入仓库地址');
      return;
    }
    submitBtn.disabled = true;
    _setSedimentKbError('sediment-kb-add-error', '');
    try {
      const categoryId = catSelect.value || undefined;
      const description = descInput.value.trim() || undefined;
      const res = await api.addSedimentKbRepo(fullName, categoryId, description);
      if (res?.error) throw new Error(res.error);
      _sedimentKbList = null;
      _sedimentKbCategories = null;
      _closeSedimentKbAddDialog();
    } catch (e) {
      _setSedimentKbError('sediment-kb-add-error', e.message);
    } finally {
      submitBtn.disabled = false;
    }
  })();
});

document.getElementById('btn-sediment-kb-manage-close').addEventListener('click', _closeSedimentKbManageDialog);
document.getElementById('sediment-kb-manage-dialog').addEventListener('click', e => {
  if (e.target === document.getElementById('sediment-kb-manage-dialog')) _closeSedimentKbManageDialog();
});
document.getElementById('btn-sediment-kb-manage-add').addEventListener('click', () => {
  void (async () => {
    const input = document.getElementById('sediment-kb-manage-new-name');
    const name = input.value.trim();
    if (!name) return;
    try {
      const res = await api.addSedimentKbCategory(name);
      if (res?.error) throw new Error(res.error);
      input.value = '';
      _sedimentKbCategories = null;
      const data = await api.fetchSedimentKbCategories();
      _sedimentKbCategories = data.categories || [];
      _renderSedimentKbManageList(_sedimentKbCategories);
      _setSedimentKbError('sediment-kb-manage-error', '');
    } catch (e) {
      _setSedimentKbError('sediment-kb-manage-error', e.message);
    }
  })();
});

document.getElementById('sediment-kb-add-url').addEventListener('keydown', e => {
  if (e.key === 'Enter') document.getElementById('btn-sediment-kb-add-submit').click();
});
document.getElementById('sediment-kb-manage-new-name').addEventListener('keydown', e => {
  if (e.key === 'Enter') document.getElementById('btn-sediment-kb-manage-add').click();
});

// ── Event listeners ────────────────────────────────────────────────────────

document.getElementById('doc-list').addEventListener('scroll', () => {
  if (state.ui.activeDate) {
    sessionStorage.setItem('cta_scroll_' + state.ui.activeDate, document.getElementById('doc-list').scrollTop);
  }
}, { passive: true });

document.getElementById('btn-edit').addEventListener('click', enterEditMode);
document.getElementById('btn-save').addEventListener('click', saveDoc);
document.getElementById('btn-cancel-edit').addEventListener('click', () => exitEditMode(false));

document.getElementById('btn-panel-commit').addEventListener('click', openCommitDialog);

document.getElementById('btn-pull').addEventListener('click', pullProject);
document.getElementById('btn-local-refresh').addEventListener('click', () => {
  const btn = document.getElementById('btn-local-refresh');
  btn.disabled = true;
  btn.textContent = '⟳ 刷新中…';
  loadIndex().finally(() => {
    btn.disabled = false;
    btn.textContent = '⟳ 本地刷新';
  });
});

// ── Fast tooltip shim ─────────────────────────────────────────────────────

(function() {
  const box = document.createElement('div');
  box.id = '_tip';
  document.body.appendChild(box);
  let timer = null;

  function show(el, x, y) {
    const text = el.dataset.tip;
    if (!text) return;
    box.textContent = text;
    const gap = 8;
    let top = y + gap;
    let left = x + gap;
    box.classList.remove('visible');
    box.style.display = 'block';
    const bw = box.offsetWidth, bh = box.offsetHeight;
    if (top + bh > window.innerHeight - 4) top = y - bh - gap;
    if (left + bw > window.innerWidth - 4) left = window.innerWidth - bw - 4;
    box.style.left = left + 'px';
    box.style.top = top + 'px';
    box.classList.add('visible');
  }

  function hide() {
    clearTimeout(timer);
    box.classList.remove('visible');
  }

  document.addEventListener('mouseover', e => {
    const el = e.target.closest('[data-tip]');
    if (!el) { hide(); return; }
    if (!el.dataset.tip) { hide(); return; }
    clearTimeout(timer);
    timer = setTimeout(() => show(el, e.clientX, e.clientY), 150);
  }, true);

  document.addEventListener('mousemove', e => {
    if (!box.classList.contains('visible')) return;
    const el = e.target.closest('[data-tip]');
    if (!el) return;
    let top = e.clientY + 8, left = e.clientX + 8;
    const bw = box.offsetWidth, bh = box.offsetHeight;
    if (top + bh > window.innerHeight - 4) top = e.clientY - bh - 8;
    if (left + bw > window.innerWidth - 4) left = window.innerWidth - bw - 4;
    box.style.left = left + 'px';
    box.style.top = top + 'px';
  }, true);

  document.addEventListener('mouseout', e => {
    const el = e.target.closest('[data-tip]');
    if (!el) return;
    if (el.contains(e.relatedTarget)) return;
    clearTimeout(timer);
    hide();
  }, true);
})();

// ── cta:reload ─────────────────────────────────────────────────────────────

document.addEventListener('cta:reload', () => loadIndex());

// ── Feed Tab ───────────────────────────────────────────────────────────────

const ARCHIVE_ELS = ['status', 'date-heading', 'doc-list'].map(id => document.getElementById(id));
const feedView = document.getElementById('feed-view');
let feedLoaded = false;

function showFeedView() {
  ARCHIVE_ELS.forEach(el => { if (el) el.style.display = 'none'; });
  feedView.style.display = '';
  document.getElementById('btn-feed').classList.add('active');
  if (!feedLoaded) {
    feedLoaded = true;
    renderFeed(feedView);
  }
}

function showArchiveView() {
  feedView.style.display = 'none';
  document.getElementById('btn-feed').classList.remove('active');
  // Restore archive elements to their natural display state
  const status = document.getElementById('status');
  const dateHeading = document.getElementById('date-heading');
  const docList = document.getElementById('doc-list');
  // Only restore status if we are not in a state where date-heading/doc-list are showing
  if (state.ui.activeDate) {
    if (status) status.style.display = 'none';
    if (dateHeading) dateHeading.style.display = '';
    if (docList) docList.style.display = '';
  } else {
    if (status) status.style.display = '';
    if (dateHeading) dateHeading.style.display = 'none';
    if (docList) docList.style.display = '';
  }
}

document.getElementById('btn-feed').addEventListener('click', () => {
  const isFeedActive = feedView.style.display !== 'none';
  if (isFeedActive) {
    showArchiveView();
  } else {
    showFeedView();
  }
});

document.getElementById('btn-settings').addEventListener('click', () => {
  _closeAllMenuDropdowns();
  openSettingsDialog();
});

document.getElementById('btn-base64').addEventListener('click', () => {
  _closeAllMenuDropdowns();
  openBase64Dialog();
});

document.getElementById('btn-qr').addEventListener('click', () => {
  _closeAllMenuDropdowns();
  openQrDialog();
});

// ── Init ───────────────────────────────────────────────────────────────────

api.fetchConfig().then(d => {
  state.ui.workbenchKnowledgeRoot = d.workbench_knowledge_root || '';
  state.ui.knowledgeCorpusRoot = d.knowledge_corpus_root || '';
  state.ui.githubUserUrl = d.github_user_url || '';
  setGithubUserUrl(d.github_user_url);
}).catch(() => {});
api.fetchTopics().then(data => {
  const descMap = {};
  const repoMap = {};
  for (const t of (data.topics || [])) {
    if (!t.repo) continue;
    const key = t.dir || t.repo.split('/')[1];
    if (t.description) descMap[key] = t.description;
    repoMap[key] = `https://github.com/${t.repo}`;
  }
  state.index.topicDescriptions = descMap;
  state.index.topicRepos = repoMap;
}).catch(() => {});
initSidebarResize();
loadIndex();
initGlobalSearch();

function registerTagsReconciledListener() {
  const onReconciled = async () => {
    await Promise.all([loadTagsRegistry(), loadAnnotationsSummary()]);
    applyListFilters();
    renderSidebar();
  };
  const tryAttach = () => {
    const listen = typeof window !== 'undefined' && window.__TAURI__?.event?.listen;
    if (typeof listen !== 'function') return false;
    void listen('tags:reconciled', onReconciled);
    return true;
  };
  if (tryAttach()) return;
  let attempts = 0;
  const timer = setInterval(() => {
    if (tryAttach() || ++attempts >= 40) clearInterval(timer);
  }, 50);
}

registerTagsReconciledListener();

document.addEventListener('cta:filter-tag', ({ detail }) => {
  if (detail?.key) selectTag(detail.key);
});

// ── Global search navigation ───────────────────────────────────────────────
document.addEventListener('cta:open-entry', ({ detail }) => {
  if (!detail?.common_path) return
  const allEntries = Object.values(state.index.data || {})
  let entry = allEntries.find(e => e.common_path === detail.common_path)
  // Fallback: detail.common_path may be a zh translation file (e.g. from a
  // stale Meilisearch index). Resolve it to the main entry via translations.zh.
  if (!entry) entry = allEntries.find(e => e.translations?.zh === detail.common_path)
  if (!entry) return
  const layer = detail.layer || entry.layers?.[0] || 'raw'
  const date = entry.created_at ? entry.created_at.slice(0, 8) : null
  if (date) selectDate(date)
  openDoc(entry, layer)
});

document.addEventListener('cta:open-kb-doc', ({ detail }) => {
  if (!detail || !detail.repo || !detail.path) return
  openKbDoc(detail)
});

// ── Skills dialog ─────────────────────────────────────────────────────────

const _SKILLS_CONTENT = {
  workbench: {
    title: '✦ Lulu Workbench Skills',
    groups: [
      {
        name: 'Dialogue Summary',
        url: 'https://github.com/lulufoo/lulu-workbench-skills/tree/main/dialogue-summary',
        items: [
          { cmd: 'dtd_raw_dialogue', name: '对话归一化', desc: '对话归一化 + digest' },
          { cmd: 'dtd_distill_dialogue', name: '蒸馏（对话体）', desc: '对话体 distilled' },
          { cmd: 'dtd_distill_compose', name: '蒸馏（合成文档）', desc: '合成文档' },
          { cmd: 'dtd_distill_topic', name: '蒸馏（子话题）', desc: '子话题 distilled' },
          { cmd: 'dtd_trace', name: '轨迹与摘要', desc: '诊断 + 轨迹' }
        ]
      },
      {
        name: 'Theme Summary',
        url: 'https://github.com/lulufoo/lulu-workbench-skills/tree/main/theme-summary',
        items: [
          { cmd: 'theme-summary', name: '总结归档', desc: '总结正文归档至 raw/ 并自动 digest' }
        ]
      },
      {
        name: 'ThemeLine',
        url: 'https://github.com/lulufoo/lulu-workbench-skills/tree/main/theme-line',
        items: [
          { cmd: 'theme-line', name: '主题线整理', desc: '视频/访谈 transcript 按主题重组为时间线大纲' }
        ]
      }
    ]
  },
  softwareDev: softwareDevSkillsContent,
  lulu: {
    title: '✦ Lulu Learning Skills',
    groups: [
      {
        name: '通用模型',
        url: 'https://github.com/lulufoo/lulu-learning-skills/tree/main/target-portrait-model',
        items: [
          { cmd: 'tpm', name: '目标画像模型', desc: '对人物/产品/组织/技术/方法论输出客观画像' }
        ]
      },
      {
        name: 'AADL · Layered Cognitive',
        url: 'https://github.com/lulufoo/lulu-learning-skills/tree/main/layered-cognitive',
        items: [
          { cmd: 'lccm', name: '分层概念认知模型', desc: '诊断认知层次（感知→理解→洞察→创造）并逐层引导深化' }
        ]
      },
      {
        name: 'AADL · Practice Exercise',
        url: 'https://github.com/lulufoo/lulu-learning-skills/tree/main/practice-exercise',
        items: [
          { cmd: 'rapm', name: '逆向应用练习模型', desc: '为目标概念设计练习任务、评审学习交付物、生成 LCCM 入口问题' }
        ]
      },
      {
        name: 'AADL · Domain Deepening',
        url: 'https://github.com/lulufoo/lulu-learning-skills/tree/main/domain-deepening',
        items: [
          { cmd: 'dp_portrait', name: '领域框架视角模型', desc: '基于权威来源生成领域客观画像' },
          { cmd: 'dp_graph',   name: '领域知识图谱模型', desc: '多轮迭代构建领域关键点网络' },
          { cmd: 'dp_role',    name: '角色视图生成模型', desc: '从知识图谱为特定角色生成关注度矩阵与学习路径' }
        ]
      },
    ]
  }
};

function _escapeAttr(str) {
  return String(str)
    .replace(/&/g, '&amp;')
    .replace(/"/g, '&quot;')
    .replace(/</g, '&lt;');
}

function _openSkillsDialog(key) {
  const data = _SKILLS_CONTENT[key];
  if (!data) return;
  document.getElementById('skills-dialog-title').textContent = data.title;
  document.getElementById('skills-dialog-body').innerHTML = data.groups.map(g => {
    const titleHtml = g.name
      ? `<div class="skill-group-title"><a class="skill-group-link" href="${g.url}" target="_blank" rel="noopener noreferrer">${g.name} ↗</a></div>`
      : '';
    const items = g.items.map(i => {
      if (typeof i === 'string') {
        return `<div class="skill-item" data-copy="${_escapeAttr(i)}" title="点击复制">${i}</div>`;
      }
      const tip = i.desc != null && i.desc !== ''
        ? _escapeAttr(i.desc)
        : '点击复制指令';
      return `<div class="skill-item skill-item-rich" data-copy="${_escapeAttr(i.cmd)}" title="${tip}">` +
        `<span class="skill-item-name">${i.name}</span>` +
        `<code class="skill-item-cmd">${i.cmd}</code>` +
        `</div>`;
    }).join('');
    return `<div class="skill-group">${titleHtml}${items}</div>`;
  }).join('');

  // Bind copy on item click
  document.getElementById('skills-dialog-body').querySelectorAll('.skill-item[data-copy]').forEach(el => {
    el.addEventListener('click', () => {
      navigator.clipboard.writeText(el.dataset.copy).then(() => {
        const orig = el.textContent;
        el.textContent = '✓ 已复制';
        setTimeout(() => { el.textContent = orig; }, 1200);
      });
    });
  });

  document.getElementById('skills-dialog').classList.add('open');
}

function _closeSkillsDialog() {
  document.getElementById('skills-dialog').classList.remove('open');
}

document.getElementById('btn-skill-workbench').addEventListener('click', () => {
  _skillsMenuDropdown.classList.remove('open');
  _openSkillsDialog('workbench');
});

document.getElementById('btn-skill-lulu').addEventListener('click', () => {
  _skillsMenuDropdown.classList.remove('open');
  _openSkillsDialog('lulu');
});

document.getElementById('btn-skill-software-dev').addEventListener('click', () => {
  _skillsMenuDropdown.classList.remove('open');
  _openSkillsDialog('softwareDev');
});

document.getElementById('btn-skills-dialog-close').addEventListener('click', _closeSkillsDialog);
document.getElementById('skills-dialog').addEventListener('click', e => {
  if (e.target === document.getElementById('skills-dialog')) _closeSkillsDialog();
});

