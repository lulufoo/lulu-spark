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

const titleCache = state.index.titleCache;

// ── Fetch index.json ───────────────────────────────────────────────────────

async function loadIndex() {
  const btn = document.getElementById('btn-refresh');
  btn.disabled = true;
  btn.textContent = '⟳ 加载中…';

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
  } finally {
    btn.disabled = false;
    btn.textContent = '⟳ 刷新';
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

// ── Event listeners ────────────────────────────────────────────────────────

document.getElementById('btn-refresh').addEventListener('click', () => {
  titleCache.clear();
  if (state.index.data) {
    for (const entry of Object.values(state.index.data)) {
      LAYERS.forEach(l => delete entry[`_unreachable_${l}`]);
    }
  }
  loadIndex();
});

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

// ── Init ───────────────────────────────────────────────────────────────────

api.fetchConfig().then(d => { state.ui.archiveRoot = d.archive_root || ''; }).catch(() => {});
loadIndex();

