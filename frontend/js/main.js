import { state, loadDiffStatus } from './state.js'
import { escHtml } from './utils.js'
import { LAYERS, setCorpusGithub } from './constants.js'
import * as api from './api.js'
import { buildGroups, renderSidebar, selectDate } from './components/sidebar.js'
import { enterEditMode, exitEditMode, saveDoc, showCommitBar, hideCommitBar, commitCurrentFile, openKbDoc, openDoc } from './components/viewer.js'
import './components/comments.js'
import './components/kb-viewer.js'
import './components/modals/delete-dialog.js'
import './components/modals/commit-dialog.js'
import './components/modals/move-dialog.js'
import { openBase64Dialog } from './components/modals/base64-dialog.js'
import { renderFeed } from './feed.js'
import { initGlobalSearch } from './components/global-search.js'
import { softwareDevSkillsContent } from './skills-software-dev-content.js'

const titleCache = state.index.titleCache;

// ── Fetch index.json ───────────────────────────────────────────────────────

async function loadIndex({ managedBtn = false } = {}) {
  try {
    const data = await api.fetchIndex();
    state.index.data = data.entries || data;
    for (const [id, entry] of Object.entries(state.index.data)) {
      entry._id = id;
    }
    state.index.groupedByDate = buildGroups(state.index.data);
    state.ui.activeTopic = null;
    state.index.filteredGroups = state.index.groupedByDate;
    renderSidebar();
    await Promise.all([loadDiffStatus(), loadAnnotationsSummary()]);
    const savedDate = sessionStorage.getItem('cta_active_date');
    const targetDate = (savedDate && state.index.filteredGroups.find(g => g.date === savedDate))
      ? savedDate
      : (state.index.filteredGroups.length > 0 ? state.index.filteredGroups[0].date : null);
    if (targetDate) selectDate(targetDate);
  } catch (e) {
    showError(`无法加载 index.json：${e.message}`);
  }
}

async function loadAnnotationsSummary() {
  try {
    const summary = await api.fetchAnnotationsSummary();
    if (!summary) return;
    state.index.annotations = summary;
    for (const entry of Object.values(state.index.data)) {
      const ann = state.index.annotations[entry.common_path];
      if (ann) {
        entry.done = ann.done || undefined;
        entry.importance = ann.importance || undefined;
        entry.links = ann.links || undefined;
        entry._comment_counts = ann.comment_counts || undefined;
      } else {
        delete entry.done;
        delete entry.importance;
        delete entry.links;
        delete entry._comment_counts;
      }
    }
  } catch {
    // Non-fatal: annotations are optional
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

// ── 列表（repo-list：全量 Git 仓库，按 type 分组 + 筛选）────────────────

function _closeRepoListDialog() {
  document.getElementById('repo-list-dialog').classList.remove('open');
}

let _repoListAll = [];
let _repoListCache = null;

const _KB_TYPES = new Set(['KNOWLEDGE_CORPUS', 'WORKBENCH_KNOWLEDGE']);

let _kbCorpusStatus = null;
let _kbCorpusStatusType = null;

async function _loadKbCorpusStatus(filterType, forceRefresh = false) {
  if (!forceRefresh && _kbCorpusStatus && _kbCorpusStatusType === filterType) {
    _renderRepoListFiltered();
    return;
  }
  _kbCorpusStatusType = filterType;
  _kbCorpusStatus = null;
  try {
    const data = await api.getKbCorpusStatus(filterType);
    if (_kbCorpusStatusType !== filterType) return;
    _kbCorpusStatus = (!data.error && data.repos) ? data.repos : [];
  } catch (e) {
    if (_kbCorpusStatusType !== filterType) return;
    _kbCorpusStatus = [];
  }
  _renderRepoListFiltered();
}

function _repoTypeKey(r) {
  return (r.type && r.type.trim()) || '未分类';
}

function _renderRepoListFiltered() {
  const filter = document.getElementById('repo-list-filter');
  const selected = filter ? filter.value : '__ALL__';
  const repos = selected === '__ALL__'
    ? _repoListAll
    : _repoListAll.filter(r => _repoTypeKey(r) === selected);

  const isKbView = _KB_TYPES.has(selected);
  const syncBtn = document.getElementById('btn-kb-corpus-sync');
  if (syncBtn) syncBtn.style.display = isKbView ? '' : 'none';

  const content = document.getElementById('repo-list-content');
  if (!repos || repos.length === 0) {
    content.innerHTML = '<div id="repo-list-loading">未找到任何仓库</div>';
    return;
  }

  const statusMap = {};
  if (isKbView && _kbCorpusStatus) {
    for (const s of _kbCorpusStatus) statusMap[s.full_name] = s;
  }

  const groups = {};
  for (const r of repos) {
    const key = _repoTypeKey(r);
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

  const showGroupTitle = selected === '__ALL__';
  const html = sortedKeys.map(key => {
    const items = groups[key].map(r => {
      const name = escHtml(r.name || r.full_name || '');
      const desc = r.description ? `<div class="repo-list-item-desc">${escHtml(r.description)}</div>` : '';
      const url = `https://github.com/${escHtml(r.full_name || r.name)}`;

      let localBadge = '';
      let syncBtnHtml = '';
      if (isKbView) {
        const st = statusMap[r.full_name];
        if (st) {
          if (st.local_exists) {
            localBadge = `<span class="repo-local-badge repo-local-ok">已克隆</span>`;
          } else {
            localBadge = `<span class="repo-local-badge repo-local-missing">未克隆</span>`;
          }
          syncBtnHtml = `<button class="repo-sync-btn" data-repo="${escHtml(r.full_name)}" data-repotype="${escHtml(selected)}">SYNC</button>`;
        }
      }

      return `<div class="repo-list-item">
        <div class="repo-list-item-info">
          <div class="repo-list-item-name">${name}${localBadge}</div>
          ${desc}
        </div>
        ${syncBtnHtml}
        <a class="repo-list-item-link" href="${url}" target="_blank" rel="noopener noreferrer">Link ↗</a>
      </div>`;
    }).join('');
    const title = showGroupTitle ? `<div class="repo-list-group-title">${escHtml(key)}</div>` : '';
    return `${title}${items}`;
  }).join('');

  content.innerHTML = html;

  content.querySelectorAll('.repo-sync-btn').forEach(btn => {
    btn.addEventListener('click', async () => {
      const repo = btn.dataset.repo;
      const repoType = btn.dataset.repotype;
      btn.disabled = true;
      btn.textContent = '…';
      try {
        let poll;
        if (repoType === 'WORKBENCH_KNOWLEDGE') {
          await api.syncWorkbenchRepo(repo);
          poll = () => api.getReindexWorkbenchStatus();
        } else {
          await api.reindexKbRepo(repo);
          poll = () => api.getReindexStatus();
        }
        for (let i = 0; i < 120; i++) {
          await new Promise(r => setTimeout(r, 2000));
          const s = await poll();
          if (!s.running) break;
        }
        _kbCorpusStatus = null;
        await _loadKbCorpusStatus(repoType, true);
      } catch (e) {
        alert(`同步失败：${e.message}`);
        btn.disabled = false;
        btn.textContent = 'SYNC';
      }
    });
  });
}

function _populateRepoListFilter(repos, preferType = null) {
  const filter = document.getElementById('repo-list-filter');
  const types = [...new Set(repos.map(_repoTypeKey))].sort((a, b) => {
    if (a === '未分类') return 1;
    if (b === '未分类') return -1;
    return a.localeCompare(b);
  });
  const selected = preferType && types.includes(preferType) ? preferType : 'KNOWLEDGE_CORPUS';
  filter.innerHTML = types.map(t => {
      const sel = t === selected ? ' selected' : '';
      return `<option value="${escHtml(t)}"${sel}>${escHtml(t)}</option>`;
    }).join('');
}

let _repoListBgRefreshing = false;

function _applyRepoListPayload(data) {
  if (!data || data.error) return false;
  const cacheTime = document.getElementById('repo-list-cache-time');
  const filter = document.getElementById('repo-list-filter');
  const prevType = filter ? filter.value : null;
  _repoListCache = data.repos || [];
  _repoListAll = _repoListCache;
  _populateRepoListFilter(_repoListAll, prevType);
  _renderRepoListFiltered();
  if (cacheTime && data.cached_at) cacheTime.textContent = `缓存于 ${data.cached_at}`;
  return true;
}

document.getElementById('repo-list-filter').addEventListener('change', () => {
  const filter = document.getElementById('repo-list-filter');
  const val = filter ? filter.value : 'KNOWLEDGE_CORPUS';
  _kbCorpusStatus = null;
  _kbCorpusStatusType = null;
  _renderRepoListFiltered();
  if (_KB_TYPES.has(val)) {
    _loadKbCorpusStatus(val);
  }
});

/** Show latest repo list from `.cache/repo-list.json` (via read API). */
async function _showRepoListFromCache() {
  try {
    const data = await api.fetchRepoList(false);
    if (_applyRepoListPayload(data)) return;
  } catch (_) {}
  _repoListAll = _repoListCache || [];
  _renderRepoListFiltered();
}

/** Kick GitHub refresh in background; update UI when cache file is written. */
async function _refreshRepoListInBackground() {
  if (_repoListBgRefreshing) return;
  _repoListBgRefreshing = true;
  const refreshBtn = document.getElementById('btn-repo-list-refresh');
  const cacheTime = document.getElementById('repo-list-cache-time');
  refreshBtn?.classList.add('spinning');
  try {
    const kick = await api.fetchRepoList(true);
    if (kick?.error) throw new Error(kick.error);
    if (kick?.status === 'running') {
      for (let i = 0; i < 120; i++) {
        await new Promise(r => setTimeout(r, 1000));
        const st = await api.getRepoListStatus();
        if (st?.status === 'error') throw new Error(st.log || '仓库列表刷新失败');
        if (st?.status !== 'running') break;
      }
      const data = await api.fetchRepoList(false);
      if (!_applyRepoListPayload(data)) throw new Error(data?.error || '仓库列表刷新失败');
    } else {
      const data = kick?.repos ? kick : await api.fetchRepoList(false);
      if (!_applyRepoListPayload(data)) throw new Error(data?.error || '仓库列表刷新失败');
    }
  } catch (e) {
    if (cacheTime) cacheTime.textContent = `刷新失败：${e.message}`;
  } finally {
    _repoListBgRefreshing = false;
    refreshBtn?.classList.remove('spinning');
  }
}

async function _loadRepoListData() {
  const content = document.getElementById('repo-list-content');
  content.innerHTML = '<div id="repo-list-loading">加载中…</div>';
  try {
    const data = await api.fetchRepoList(false);
    if (data.error) throw new Error(data.error);
    _applyRepoListPayload(data);
  } catch (e) {
    content.innerHTML = `<div id="repo-list-loading" style="color:#cf222e">加载失败：${escHtml(e.message)}</div>`;
  }
}

document.getElementById('btn-repo-list').addEventListener('click', async () => {
  _repoMenuDropdown.classList.remove('open');
  document.getElementById('repo-list-dialog').classList.add('open');
  await _loadRepoListData();
  const filter = document.getElementById('repo-list-filter');
  const val = filter ? filter.value : 'KNOWLEDGE_CORPUS';
  if (_KB_TYPES.has(val)) _loadKbCorpusStatus(val);
});

document.getElementById('btn-repo-list-refresh').addEventListener('click', () => {
  void (async () => {
    await _showRepoListFromCache();
    const filter = document.getElementById('repo-list-filter');
    const val = filter ? filter.value : 'KNOWLEDGE_CORPUS';
    if (_KB_TYPES.has(val)) _loadKbCorpusStatus(val);
    await _refreshRepoListInBackground();
    const val2 = filter ? filter.value : 'KNOWLEDGE_CORPUS';
    if (_KB_TYPES.has(val2)) _loadKbCorpusStatus(val2, true);
  })();
});

document.getElementById('btn-repo-list-close').addEventListener('click', _closeRepoListDialog);
document.getElementById('repo-list-dialog').addEventListener('click', e => {
  if (e.target === document.getElementById('repo-list-dialog')) _closeRepoListDialog();
});

document.getElementById('btn-kb-corpus-sync').addEventListener('click', async () => {
  const btn = document.getElementById('btn-kb-corpus-sync');
  btn.disabled = true;
  btn.textContent = '⊙ 同步中…';
  try {
    const filter = document.getElementById('repo-list-filter');
    const filterType = filter ? filter.value : 'KNOWLEDGE_CORPUS';
    let poll;
    if (filterType === 'WORKBENCH_KNOWLEDGE') {
      await api.syncWorkbenchCorpus();
      poll = () => api.getReindexWorkbenchStatus();
    } else {
      await api.syncKnowledgeCorpus();
      poll = () => api.getReindexStatus();
    }
    for (let i = 0; i < 120; i++) {
      await new Promise(r => setTimeout(r, 2000));
      const s = await poll();
      if (!s.running) break;
    }
    _kbCorpusStatus = null;
    const filter2 = document.getElementById('repo-list-filter');
    const val = filter2 ? filter2.value : 'KNOWLEDGE_CORPUS';
    await _loadKbCorpusStatus(val, true);
  } catch (e) {
    alert(`全量同步失败：${e.message}`);
  } finally {
    btn.disabled = false;
    btn.textContent = '⊙ 全量同步';
  }
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

document.getElementById('btn-panel-commit').addEventListener('click', showCommitBar);
document.getElementById('btn-commit-file').addEventListener('click', commitCurrentFile);
document.getElementById('btn-skip-commit').addEventListener('click', hideCommitBar);
document.getElementById('md-commit-msg').addEventListener('keydown', e => {
  if (e.key === 'Enter') commitCurrentFile();
});

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

document.getElementById('btn-base64').addEventListener('click', () => {
  _closeAllMenuDropdowns();
  openBase64Dialog();
});

// ── Init ───────────────────────────────────────────────────────────────────

api.fetchConfig().then(d => {
  state.ui.archiveRoot = d.archive_root || '';
  state.ui.kbRoot = d.kb_root || '';
  state.ui.corpusGithub = d.corpus_github || '';
  setCorpusGithub(d.corpus_github);
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
loadIndex();
initGlobalSearch();

// ── Global search navigation ───────────────────────────────────────────────
document.addEventListener('cta:open-entry', ({ detail }) => {
  if (!detail?.common_path) return
  const entry = Object.values(state.index.data || {})
    .find(e => e.common_path === detail.common_path)
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
        url: 'https://github.com/lulufoo/lulu-skills/tree/main/target-portrait-model',
        items: [
          { cmd: 'tpm', name: '目标画像模型', desc: '对人物/产品/组织/技术/方法论输出客观画像' }
        ]
      },
      {
        name: 'AADL · Layered Cognitive',
        url: 'https://github.com/lulufoo/lulu-skills/tree/main/ai-assisted-domain-learning/layered-cognitive',
        items: [
          { cmd: 'lccm', name: '分层概念认知模型', desc: '诊断认知层次（感知→理解→洞察→创造）并逐层引导深化' }
        ]
      },
      {
        name: 'AADL · Practice Exercise',
        url: 'https://github.com/lulufoo/lulu-skills/tree/main/ai-assisted-domain-learning/practice-exercise',
        items: [
          { cmd: 'rapm', name: '逆向应用练习模型', desc: '为目标概念设计练习任务、评审学习交付物、生成 LCCM 入口问题' }
        ]
      },
      {
        name: 'AADL · Domain Deepening',
        url: 'https://github.com/lulufoo/lulu-skills/tree/main/ai-assisted-domain-learning/domain-deepening',
        items: [
          { cmd: 'dp_portrait', name: '领域框架视角模型', desc: '基于权威来源生成领域客观画像' },
          { cmd: 'dp_graph',   name: '领域知识图谱模型', desc: '多轮迭代构建领域关键点网络与知识图谱' },
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

