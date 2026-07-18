import { state, loadDiffStatus, mergeAnnotations } from './state.js'
import { escHtml } from './utils.js'
import { LAYERS, setGithubUserUrl } from './constants.js'
import * as api from './api.js'
import { buildGroups, renderSidebar, selectDate, applyListFilters, selectTag } from './components/sidebar.js'
import { initSidebarResize } from './components/sidebar-resize.js'
import { enterEditMode, exitEditMode, saveDoc, openCommitDialog, openDoc, openCreateNote } from './components/viewer.js'
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
import { initRouter, navigate } from './router/index.js'
import { mountReadLaterAssistantWidget } from './read-later-assistant.js'
import { mountPlanTaskAssistantWidget } from './plan-task-assistant.js'
import { mountNoteAssistantWidget } from './note-assistant.js'
import { mountBuildersAssistantWidget } from './builders-assistant.js'
import { openReadLaterDialog } from './components/modals/read-later-dialog.js'
import { applySearchNavChrome } from './nav-chrome.js'
import { initWorkbenchSearch } from './components/workbench-search.js'
import { initCorpusSearch } from './components/corpus-search.js'
import { mountCorpusDocList } from './components/corpus-doc-list.js'
import { mountHomeHub } from './components/home-hub.js'
import { mountPlanTaskSplit } from './plan-task/index.js'
import { initHeaderSync, clearHeaderSyncCorpusContext } from './header-sync.js'
import { softwareDevSkillsContent } from './skills-software-dev-content.js'
import { workbenchSkillsContent } from './skills-workbench-content.js'
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
    const savedDate = sessionStorage.getItem('cta_active_date');
    const targetDate = (savedDate && state.index.filteredGroups.find(g => g.date === savedDate))
      ? savedDate
      : (state.index.filteredGroups.length > 0 ? state.index.filteredGroups[0].date : null);
    if (targetDate) selectDate(targetDate);
  } catch (e) {
    showError(`Could not load index.json: ${e.message}`);
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
  retryBtn.textContent = 'Retry';
  retryBtn.addEventListener('click', loadIndex);
  errDiv.appendChild(retryBtn);
  status.innerHTML = '';
  status.appendChild(errDiv);
  document.getElementById('date-heading').style.display = 'none';
  document.getElementById('doc-list').innerHTML = '';
}

// ── Pull project ───────────────────────────────────────────────────────────

async function pullProject() {
  const btn = document.getElementById('btn-pull');
  btn.disabled = true;
  btn.textContent = 'Updating…';
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
    alert(`Update failed: ${e.message}`);
  } finally {
    btn.disabled = false;
    btn.textContent = '↓ Update project';
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
    content.innerHTML = '<div id="repo-list-loading">No repositories found</div>';
    return;
  }

  const categories = _sedimentKbCategories || [];
  const statusMap = {};
  if (_kbCorpusStatus) {
    for (const s of _kbCorpusStatus) statusMap[s.full_name] = s;
  }

  const groups = {};
  for (const r of repos) {
    const key = r.category_name || 'Uncategorized';
    if (!groups[key]) groups[key] = [];
    groups[key].push(r);
  }
  for (const key of Object.keys(groups)) {
    groups[key].sort((a, b) => (a.name || '').localeCompare(b.name || '', 'en', { sensitivity: 'base' }));
  }

  const sortedKeys = Object.keys(groups).sort((a, b) => {
    if (a === 'Uncategorized') return 1;
    if (b === 'Uncategorized') return -1;
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
          ? `<span class="repo-local-badge repo-local-ok">Cloned</span>`
          : `<span class="repo-local-badge repo-local-missing">Not cloned</span>`;
        if (_kbCorpusDiffStatus?.get(r.full_name) === true) {
          diffBtnHtml = `<button class="repo-diff-badge" data-repo="${escHtml(r.full_name)}" title="View local changes">✎</button>`;
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
          <button type="button" class="sediment-kb-delete-btn" data-repo="${escHtml(r.full_name)}" title="Remove from curated list">Delete</button>
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
        alert(`Sync failed: ${e.message}`);
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
    alert(`Failed to update category: ${e.message}`);
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
    alert(`Delete failed: ${e.message}`);
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
      content.innerHTML = `<div id="repo-list-loading" style="color:#cf222e">Failed to load: ${escHtml(_sedimentKbError)}</div>`;
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
  if (title) title.textContent = '☰ Knowledge list';
  document.getElementById('repo-list-dialog').classList.add('open');
  const content = document.getElementById('repo-list-content');
  content.innerHTML = '<div id="repo-list-loading">Loading…</div>';
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
      '<option value="uncategorized">Uncategorized</option>';
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
      : `<button type="button" class="sediment-kb-cat-delete-btn" data-id="${escHtml(c.id)}">Delete</button>`;
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
      _setSedimentKbError('sediment-kb-add-error', 'Enter repository URL');
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

initHeaderSync({ pullProject, loadIndex });

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

// ── Feed Tab (legacy #feed-view kept hidden; Builders entry is FAB-only) ───

const feedView = document.getElementById('feed-view');

let unmountCorpusDocList = null;
let corpusDocListRepo = '';
let unmountHomeHub = null;
let unmountPlanTaskSplit = null;

function updateNavChrome(routeName) {
  const onHome = routeName === 'home';
  const homeTitle = document.getElementById('btn-nav-home-title');
  const homeNav = document.getElementById('btn-nav-home');
  if (homeTitle) homeTitle.hidden = !onHome;
  if (homeNav) homeNav.hidden = onHome;
  applySearchNavChrome(routeName);
}

function wrapRouteMount(routeName, mountFn) {
  return (route) => {
    updateNavChrome(routeName);
    if (routeName === 'workbench') initWorkbenchSearch();
    if (routeName === 'corpus-doc') initCorpusSearch();
    return mountFn(route);
  };
}

function hideHomeView() {
  const homeView = document.getElementById('home-view');
  if (homeView) homeView.style.display = 'none';
}

function hideCorpusDocView() {
  const docView = document.getElementById('corpus-doc-view');
  if (docView) docView.style.display = 'none';
  const layout = document.querySelector('.layout');
  if (layout) layout.style.display = '';
}

function hideReadLaterView() {
  const readLaterView = document.getElementById('read-later-view');
  if (readLaterView) readLaterView.style.display = 'none';
}

function hidePlanTasksView() {
  const planTasksView = document.getElementById('plan-tasks-view');
  if (planTasksView) planTasksView.style.display = 'none';
}

function mountHomeRoute() {
  clearHeaderSyncCorpusContext();
  unmountCorpusDocList?.();
  unmountCorpusDocList = null;
  corpusDocListRepo = '';
  hideCorpusDocView();
  hideReadLaterView();
  hidePlanTasksView();

  if (feedView) feedView.style.display = 'none';

  const layout = document.querySelector('.layout');
  if (layout) layout.style.display = 'none';

  const homeView = document.getElementById('home-view');
  if (!homeView) return;
  homeView.style.display = '';

  unmountPlanTaskSplit?.();
  unmountPlanTaskSplit = null;
  unmountHomeHub?.();
  unmountHomeHub = mountHomeHub(homeView, { navigate, openReadLater: openReadLaterDialog });
}

function mountCorpusDocRoute(route) {
  unmountHomeHub?.();
  unmountHomeHub = null;
  unmountPlanTaskSplit?.();
  unmountPlanTaskSplit = null;
  hideHomeView();
  hideReadLaterView();
  hidePlanTasksView();

  const layout = document.querySelector('.layout');
  if (layout) layout.style.display = 'none';

  const docView = document.getElementById('corpus-doc-view');
  if (!docView) return;
  docView.style.display = '';

  const repo = route?.params?.repo || '';
  const initialPath = route?.params?.path || '';

  if (unmountCorpusDocList && corpusDocListRepo === repo) {
    // Same repo: update path in-place. Do not remount — remount resets expanded tree state.
    void unmountCorpusDocList.navigateToPath?.(initialPath);
    return;
  }

  unmountCorpusDocList?.();
  unmountCorpusDocList = null;
  corpusDocListRepo = '';

  unmountCorpusDocList = mountCorpusDocList(docView, { repo, navigate, initialPath });
  corpusDocListRepo = unmountCorpusDocList.repo ?? repo;
}

function mountReadLaterRoute() {
  mountHomeRoute();
  openReadLaterDialog();
}

function mountPlanTasksRoute(route) {
  clearHeaderSyncCorpusContext();
  unmountHomeHub?.();
  unmountHomeHub = null;
  unmountCorpusDocList?.();
  unmountCorpusDocList = null;
  corpusDocListRepo = '';
  hideHomeView();
  hideCorpusDocView();
  hideReadLaterView();

  if (feedView) feedView.style.display = 'none';

  const layout = document.querySelector('.layout');
  if (layout) layout.style.display = 'none';

  const planTasksView = document.getElementById('plan-tasks-view');
  if (!planTasksView) return;
  planTasksView.style.display = '';

  unmountPlanTaskSplit?.();
  const masterId = route?.params?.master ?? '';
  const subId = route?.params?.sub ?? '';
  const mounted = mountPlanTaskSplit(planTasksView, {
    masterId,
    subId,
    navigate,
  });
  unmountPlanTaskSplit = mounted.unmount;
}

function mountWorkbench() {
  clearHeaderSyncCorpusContext();
  unmountHomeHub?.();
  unmountHomeHub = null;
  unmountPlanTaskSplit?.();
  unmountPlanTaskSplit = null;
  hideHomeView();
  unmountCorpusDocList?.();
  unmountCorpusDocList = null;
  corpusDocListRepo = '';
  hideCorpusDocView();
  hideReadLaterView();
  hidePlanTasksView();

  if (feedView) feedView.style.display = 'none';
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

initRouter({
  workbench: wrapRouteMount('workbench', () => mountWorkbench()),
  home: wrapRouteMount('home', mountHomeRoute),
  'corpus-doc': wrapRouteMount('corpus-doc', mountCorpusDocRoute),
  'read-later': wrapRouteMount('read-later', mountReadLaterRoute),
  'plan-tasks': wrapRouteMount('plan-tasks', mountPlanTasksRoute),
}, { fallback: '#/home' });

const readLaterAssistant = mountReadLaterAssistantWidget(document.body, {
  navigate,
  openReadLater: openReadLaterDialog,
});
const planTaskAssistant = mountPlanTaskAssistantWidget(document.body, { navigate });

let noteAssistant = null;
function closeNoteAssistantPanel() {
  noteAssistant?.setOpen(false);
}

function openCreateNoteFromFab(opts = {}) {
  closeNoteAssistantPanel();
  const temp_id =
    opts?.temp_id ||
    (typeof crypto !== 'undefined' && typeof crypto.randomUUID === 'function'
      ? crypto.randomUUID()
      : `note-${Date.now()}`);
  return openCreateNote({ temp_id });
}

noteAssistant = mountNoteAssistantWidget(document.body, {
  openCreateNote: openCreateNoteFromFab,
});

const buildersAssistant = mountBuildersAssistantWidget(document.body);

document.addEventListener(
  'click',
  (event) => {
    const target = event.target;
    if (!(target instanceof Element)) return;
    if (target.closest('.builders-entry-fab')) {
      readLaterAssistant?.setOpen(false);
      planTaskAssistant?.setOpen(false);
      noteAssistant?.setOpen(false);
      return;
    }
    if (target.closest('.rl-assistant-fab')) {
      planTaskAssistant.setOpen(false);
      noteAssistant?.setOpen(false);
      buildersAssistant?.setOpen(false);
      return;
    }
    if (target.closest('.pt-assistant-fab')) {
      readLaterAssistant.setOpen(false);
      noteAssistant?.setOpen(false);
      buildersAssistant?.setOpen(false);
      return;
    }
    if (target.closest('.note-assistant-fab')) {
      readLaterAssistant.setOpen(false);
      planTaskAssistant.setOpen(false);
      buildersAssistant?.setOpen(false);
    }
  },
  true,
);

document.getElementById('btn-edit')?.addEventListener(
  'click',
  () => {
    closeNoteAssistantPanel();
  },
  true,
);

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
  closeNoteAssistantPanel();
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
  navigate('#/corpus/' + encodeURIComponent(detail.repo) + '?path=' + encodeURIComponent(detail.path))
});

// ── Skills dialog ─────────────────────────────────────────────────────────

const _SKILLS_CONTENT = {
  workbench: workbenchSkillsContent,
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
        return `<div class="skill-item" data-copy="${_escapeAttr(i)}" title="Click to copy">${i}</div>`;
      }
      const tip = i.desc != null && i.desc !== ''
        ? _escapeAttr(i.desc)
        : 'Click to copy command';
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
        el.textContent = '✓ Copied';
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

