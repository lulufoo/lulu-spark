// @ts-nocheck — ported from JS; state shapes stay unchecked like checkJs:false.
import type { ReactNode } from 'react';
import { state } from '../host/state.ts'
import * as api from '../host/api.ts'
import { renderToHtml } from '../island.ts';

// ── Settle Dialog ─────────────────────────────────────────────────────────

let _settleCtx = null;
let _topicsCache = null;
let _checkTimer = null;

async function _getTopics() {
  if (_topicsCache) return _topicsCache;
  _topicsCache = await api.fetchTopics();
  return _topicsCache;
}

function _deriveRepo(commonPath, topics) {
  const projectDir = commonPath.split('/')[0];
  for (const t of (topics.topics || [])) {
    if (!t.repo) continue;
    const repoName = t.repo.split('/')[1];
    if (repoName === projectDir || t.dir === projectDir) return t.repo;
  }
  return null;
}

function _extractSlug(commonPath) {
  const filename = commonPath.split('/').pop() || '';
  // Remove .md extension and leading 12-digit timestamp
  return filename.replace(/\.md$/, '').replace(/^\d{12}-/, '');
}

function _nowTs() {
  const now = new Date();
  const utc8 = new Date(now.getTime() + 8 * 60 * 60 * 1000);
  const pad = n => String(n).padStart(2, '0');
  return `${utc8.getUTCFullYear()}${pad(utc8.getUTCMonth() + 1)}${pad(utc8.getUTCDate())}${pad(utc8.getUTCHours())}${pad(utc8.getUTCMinutes())}`;
}

function _updatePreview() {
  const ts = _nowTs();
  document.getElementById('settle-filename-ts').textContent = ts;
  _scheduleCheck();
}

function _scheduleCheck() {
  clearTimeout(_checkTimer);
  const warn = document.getElementById('settle-file-warn');
  if (warn) warn.textContent = '';
  _checkTimer = setTimeout(_checkExistence, 600);
}

async function _checkExistence() {
  if (!_settleCtx) return;
  const warn = document.getElementById('settle-file-warn');
  if (!warn) return;

  const sel = document.getElementById('settle-theme-select');
  const inp = document.getElementById('settle-theme-input');
  const slug = document.getElementById('settle-slug').value.trim();
  const docTheme = inp.style.display !== 'none' ? inp.value.trim() : sel.value;

  if (!slug || !docTheme || docTheme === '__new__') { warn.textContent = ''; return; }

  const ts = _nowTs();
  const filename = `${ts}-${slug}.md`;
  const filePath = docTheme === '.' ? filename : `${docTheme}/${filename}`;

  try {
    const data = await api.checkFileExists(_settleCtx.repo, filePath);
    if (data.exists) {
      warn.innerHTML = renderToHtml(
        <span style={{ color: '#cf222e', fontSize: 11 }}>⚠ File already exists: {filePath}</span>,
      );
    } else {
      warn.textContent = '';
    }
  } catch (_) {
    // Ignore existence check failures silently
  }
}

function paintSelectOptions(sel, node: ReactNode) {
  sel.innerHTML = renderToHtml(node);
}

function paintSettleResult(el, node: ReactNode) {
  el.innerHTML = renderToHtml(node);
}

export async function openSettleDialog(comment, layer, entry) {
  let topics;
  try {
    topics = await _getTopics();
  } catch (e) {
    alert(`Could not load topics.json: ${e.message}`);
    return;
  }
  const repo = _deriveRepo(entry.common_path, topics);
  if (!repo) {
    alert(`Could not find GitHub repository for ${entry.common_path.split('/')[0]}`);
    return;
  }

  _settleCtx = { comment, layer, entry, repo };

  document.getElementById('settle-repo-display').textContent = repo;
  document.getElementById('settle-content').value = comment.text;
  document.getElementById('settle-slug').value = _extractSlug(entry.common_path);
  document.getElementById('settle-result').textContent = '';
  document.getElementById('settle-theme-input').style.display = 'none';
  document.getElementById('settle-theme-input').value = '';

  const btn = document.getElementById('btn-settle-submit');
  btn.disabled = false;
  btn.textContent = 'Push';

  const sel = document.getElementById('settle-theme-select');
  sel.style.display = '';
  paintSelectOptions(sel, <option value="">Loading folders…</option>);
  sel.disabled = true;

  document.getElementById('settle-file-warn').textContent = '';
  _updatePreview();
  document.getElementById('settle-dialog').classList.add('open');

  api.fetchRepoDirs(repo).then(data => {
    sel.disabled = false;
    if (data.error) {
      paintSelectOptions(sel, <option value="">Failed to load: {data.error}</option>);
      return;
    }
    const dirs = data.dirs || [];
    const hint = entry.common_path.split('/')[1] || '';
    paintSelectOptions(
      sel,
      <>
        <option value=".">. (root)</option>
        {dirs.map((d) => (
          <option key={d} value={d}>{d}</option>
        ))}
        <option value="__new__">＋ New folder…</option>
      </>,
    );
    if (hint && dirs.includes(hint)) sel.value = hint;
  }).catch(e => {
    sel.disabled = false;
    paintSelectOptions(sel, <option value="">Failed to load: {e.message}</option>);
  });
}

export function closeSettleDialog() {
  clearTimeout(_checkTimer);
  document.getElementById('settle-dialog').classList.remove('open');
  _settleCtx = null;
}

async function _doSettle() {
  if (!_settleCtx) return;
  const { comment, layer, entry } = _settleCtx;

  const sel = document.getElementById('settle-theme-select');
  const inp = document.getElementById('settle-theme-input');
  const docTheme = inp.style.display !== 'none' ? inp.value.trim() : sel.value;

  if (!docTheme || docTheme === '__new__') {
    alert('Choose or enter target folder');
    return;
  }

  const slug = document.getElementById('settle-slug').value.trim();
  if (!slug) { alert('Enter filename'); return; }
  if (!/^[a-z0-9][a-z0-9-]*$/.test(slug)) {
    alert('Filename may only contain lowercase letters, numbers, and hyphens, and cannot start with a hyphen');
    return;
  }

  const content = document.getElementById('settle-content').value.trim();
  if (!content) { alert('Body cannot be empty'); return; }

  const btn = document.getElementById('btn-settle-submit');
  btn.disabled = true;
  btn.textContent = 'Pushing…';
  document.getElementById('settle-result').textContent = '';

  const resultEl = document.getElementById('settle-result');

  try {
    const data = await api.settleComment(
      entry.common_path, comment.id, layer, docTheme, slug, content
    );
    if (data.error) {
      paintSettleResult(
        resultEl,
        <span style={{ color: '#cf222e' }}>Failed: {data.error}</span>,
      );
      btn.disabled = false;
      btn.textContent = 'Push';
      return;
    }

    // Dispatch success event — comments.js handles state update + re-render
    document.dispatchEvent(new CustomEvent('settle:done', {
      detail: { commentId: comment.id, layer, entry, url: data.url }
    }));

    const warns = Array.isArray(data.warn) ? data.warn : (data.warn ? [data.warn] : []);
    paintSettleResult(
      resultEl,
      <>
        <span style={{ color: '#1a7f37' }}>✓ Pushed</span>
        {'　'}
        <a href={data.url} target="_blank" style={{ fontSize: 11, wordBreak: 'break-all' }}>{data.url}</a>
        {warns.length ? (
          <>
            <br />
            <span style={{ color: '#9a6700', fontSize: 11 }}>⚠ {warns.join('；')}</span>
          </>
        ) : null}
      </>,
    );
    btn.textContent = 'Done';
    setTimeout(closeSettleDialog, 2500);
  } catch (e) {
    paintSettleResult(
      resultEl,
      <span style={{ color: '#cf222e' }}>Failed: {e.message}</span>,
    );
    btn.disabled = false;
    btn.textContent = 'Push';
  }
}

// ── Event listeners ────────────────────────────────────────────────────────

document.getElementById('btn-settle-submit').addEventListener('click', _doSettle);
document.getElementById('btn-settle-cancel').addEventListener('click', closeSettleDialog);
document.getElementById('settle-dialog').addEventListener('click', e => {
  if (e.target === document.getElementById('settle-dialog')) closeSettleDialog();
});
document.getElementById('settle-slug').addEventListener('input', _updatePreview);
document.getElementById('settle-theme-select').addEventListener('change', e => {
  const inp = document.getElementById('settle-theme-input');
  if (e.target.value === '__new__') {
    inp.style.display = '';
    inp.value = '';
    inp.focus();
  } else {
    inp.style.display = 'none';
    _scheduleCheck();
  }
});
document.getElementById('settle-theme-input').addEventListener('input', _scheduleCheck);
