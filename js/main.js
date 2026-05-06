import { state, loadDiffStatus } from './state.js'
import { escHtml } from './utils.js'
import { LAYERS } from './constants.js'
import * as api from './api.js'
import { buildGroups, renderSidebar, selectDate } from './components/sidebar.js'
import { enterEditMode, exitEditMode, saveDoc, showCommitBar, hideCommitBar, commitCurrentFile } from './components/viewer.js'
import './components/comments.js'
import './components/modals/delete-dialog.js'
import './components/modals/commit-dialog.js'
import './components/modals/move-dialog.js'
import { renderFeed } from './feed.js'

const titleCache = state.index.titleCache;

// ── Fetch index.json ───────────────────────────────────────────────────────

async function loadIndex({ managedBtn = false } = {}) {
  try {
    const data = await api.fetchIndex();
    state.index.data = data.entries || data;
    state.index.groupedByDate = buildGroups(state.index.data);
    renderSidebar();
    await Promise.all([loadDiffStatus(), loadAnnotationsSummary()]);
    const savedDate = sessionStorage.getItem('cta_active_date');
    const targetDate = (savedDate && state.index.groupedByDate.find(g => g.date === savedDate))
      ? savedDate
      : (state.index.groupedByDate.length > 0 ? state.index.groupedByDate[0].date : null);
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

async function _runUpdateTopics(mode, checkRepo = '') {
  try {
    const data = await api.updateTopics(mode, checkRepo);
    if (data.error) throw new Error(data.error);
    return data;
  } catch (e) {
    throw e;
  }
}

// ── Repo menu dropdown ─────────────────────────────────────────────────────

const _repoMenuWrap = document.getElementById('repo-menu-wrap');
const _repoMenuDropdown = document.getElementById('repo-menu-dropdown');

document.getElementById('btn-repo-menu').addEventListener('click', (e) => {
  e.stopPropagation();
  _repoMenuDropdown.classList.toggle('open');
});
document.addEventListener('click', () => _repoMenuDropdown.classList.remove('open'));
_repoMenuDropdown.addEventListener('click', e => e.stopPropagation());

// ── 刷新描述（fast）──────────────────────────────────────────────────────────

document.getElementById('btn-repo-refresh').addEventListener('click', async () => {
  _repoMenuDropdown.classList.remove('open');
  const btn = document.getElementById('btn-repo-menu');
  btn.disabled = true;
  btn.textContent = '⚙ 刷新中…';
  try {
    await _runUpdateTopics('fast');
    await _reloadTopicsIntoState();
    titleCache.clear();
    if (state.index.data) {
      for (const entry of Object.values(state.index.data)) {
        LAYERS.forEach(l => delete entry[`_unreachable_${l}`]);
      }
    }
    await loadIndex({ managedBtn: true });
  } catch (e) {
    alert(`刷新失败：${e.message}`);
  } finally {
    btn.disabled = false;
    btn.textContent = '⚙ 仓库';
  }
});

// ── 全量扫描（rediscover）───────────────────────────────────────────────────

document.getElementById('btn-repo-rediscover').addEventListener('click', async () => {
  _repoMenuDropdown.classList.remove('open');
  const btn = document.getElementById('btn-repo-menu');
  btn.disabled = true;
  btn.textContent = '⚙ 扫描中…';
  try {
    await _runUpdateTopics('rediscover');
    await _reloadTopicsIntoState();
    titleCache.clear();
    await loadIndex({ managedBtn: true });
  } catch (e) {
    alert(`全量扫描失败：${e.message}`);
  } finally {
    btn.disabled = false;
    btn.textContent = '⚙ 仓库';
  }
});

// ── 添加仓库（check-repo）──────────────────────────────────────────────────

document.getElementById('btn-repo-add').addEventListener('click', () => {
  _repoMenuDropdown.classList.remove('open');
  document.getElementById('add-repo-input').value = '';
  document.getElementById('add-repo-result').textContent = '';
  document.getElementById('add-repo-result').style.color = '';
  document.getElementById('btn-add-repo-ok').disabled = false;
  document.getElementById('btn-add-repo-ok').textContent = '确认添加';
  document.getElementById('add-repo-dialog').classList.add('open');
  document.getElementById('add-repo-input').focus();
});

document.getElementById('btn-add-repo-cancel').addEventListener('click', () => {
  document.getElementById('add-repo-dialog').classList.remove('open');
});
document.getElementById('add-repo-dialog').addEventListener('click', (e) => {
  if (e.target === document.getElementById('add-repo-dialog'))
    document.getElementById('add-repo-dialog').classList.remove('open');
});

document.getElementById('btn-add-repo-ok').addEventListener('click', async () => {
  const repo = document.getElementById('add-repo-input').value.trim();
  const result = document.getElementById('add-repo-result');
  const okBtn = document.getElementById('btn-add-repo-ok');
  if (!repo || !/^[A-Za-z0-9_.-]+\/[A-Za-z0-9_.-]+$/.test(repo)) {
    result.style.color = '#cf222e';
    result.textContent = '格式错误，请输入 owner/repo';
    return;
  }
  okBtn.disabled = true;
  okBtn.textContent = '检测中…';
  result.style.color = '#57606a';
  result.textContent = '正在检测仓库类型并更新…';
  try {
    const data = await _runUpdateTopics('fast', repo);
    result.style.color = '#1a7f37';
    result.textContent = `✓ 已添加 ${repo}`;
    await _reloadTopicsIntoState();
    setTimeout(() => {
      document.getElementById('add-repo-dialog').classList.remove('open');
    }, 1200);
  } catch (e) {
    result.style.color = '#cf222e';
    result.textContent = `✗ ${e.message}`;
    okBtn.disabled = false;
    okBtn.textContent = '确认添加';
  }
});

document.getElementById('add-repo-input').addEventListener('keydown', e => {
  if (e.key === 'Enter') document.getElementById('btn-add-repo-ok').click();
  if (e.key === 'Escape') document.getElementById('add-repo-dialog').classList.remove('open');
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

// ── Init ───────────────────────────────────────────────────────────────────

api.fetchConfig().then(d => { state.ui.archiveRoot = d.archive_root || ''; }).catch(() => {});
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

