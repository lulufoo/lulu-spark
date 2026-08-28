// @ts-nocheck — DOM wiring stays unchecked like checkJs:false.
import { state, loadDiffStatus } from '../host/state.ts';
import { getActivePath } from '../corpus/corpus-path.ts';
import { escHtml } from '../shared/utils.ts';
import * as api from '../host/api.ts';
import { updateDiffInDOM } from './cards.tsx';
import { initMermaid } from '../shared/mermaid-render.ts';
import { openKbDoc } from '../corpus/corpus-viewer.ts';
import { closeCommitDialog, hidePendingBadge, openCommitDialog } from './viewer/commit.tsx';
import { initHighlightUI } from './viewer/body.tsx';
import { enterEditMode, exitEditMode, openDoc, saveDoc, switchLang } from './viewer/doc.tsx';
import { closeModal, openCreateNote } from './viewer/create.ts';

export { openKbDoc };
export { showPendingBadge, hidePendingBadge, openCommitDialog } from './viewer/commit.tsx';
export { renderDocBody } from './viewer/body.tsx';
export { enterEditMode, exitEditMode, openDoc, saveDoc, switchLang };
export { closeModal, openCreateNote };

initMermaid();

document.getElementById('md-close').addEventListener('click', () => { void closeModal(); });
document.addEventListener('keydown', e => {
  if (e.key === 'Escape') {
    if (document.getElementById('comment-dialog').classList.contains('open')) return;
    if (document.getElementById('md-commit-dialog').classList.contains('open')) return;
    void closeModal();
  }
});

document.getElementById('md-btn-commit-cancel').addEventListener('click', () => {
  closeCommitDialog();
});

document.getElementById('md-commit-dialog').addEventListener('click', e => {
  if (e.target === document.getElementById('md-commit-dialog')) closeCommitDialog();
});

document.getElementById('md-btn-commit-ok').addEventListener('click', async () => {
  const btn = document.getElementById('md-btn-commit-ok');
  const resultEl = document.getElementById('md-commit-dialog-result');
  const msg = document.getElementById('md-commit-dialog-msg').value.trim() || 'update: edit via viewer';
  btn.disabled = true;
  resultEl.textContent = 'Committing…';
  resultEl.style.color = '#8c959f';
  try {
    const data = await api.commitFiles(msg);
    if (data.error) throw new Error(data.error);
    resultEl.style.color = '#1a7f37';
    resultEl.textContent = '✓ Pushed!';
    await loadDiffStatus();
    updateDiffInDOM();
    hidePendingBadge();
    setTimeout(() => closeCommitDialog(), 1500);
  } catch (e) {
    resultEl.style.color = '#cf222e';
    resultEl.textContent = `✗ ${e.message}`;
    btn.disabled = false;
  }
});

document.getElementById('md-btn-revert-all').addEventListener('click', async () => {
  const btn = document.getElementById('md-btn-revert-all');
  if (!btn.classList.contains('confirm')) {
    btn.classList.add('confirm');
    btn.textContent = 'Revert all changes?';
    setTimeout(() => {
      btn.classList.remove('confirm');
      btn.textContent = 'Revert all changes';
    }, 3000);
    return;
  }
  btn.disabled = true;
  try {
    const data = await api.revertFile('', '');
    if (data.error) throw new Error(data.error);
    await loadDiffStatus();
    updateDiffInDOM();
    hidePendingBadge();
    closeCommitDialog();
  } catch (e) {
    document.getElementById('md-commit-dialog-result').textContent = `Revert failed: ${e.message}`;
    btn.disabled = false;
  } finally {
    btn.classList.remove('confirm');
    btn.textContent = 'Revert all changes';
  }
});

document.getElementById('md-commit-file-list').addEventListener('click', async e => {
  const revertBtn = e.target.closest?.('.kb-revert-btn');
  if (!revertBtn) return;
  const path = revertBtn.dataset.path;
  const type = revertBtn.dataset.type;
  revertBtn.disabled = true;
  try {
    const data = await api.revertFile(path, type);
    if (data.error) throw new Error(data.error);
    await openCommitDialog();
  } catch (e_) {
    revertBtn.disabled = false;
    document.getElementById('md-commit-dialog-result').textContent = `Revert failed: ${escHtml(e_.message)}`;
  }
});

document.getElementById('btn-copy-http').addEventListener('click', () => {
  const url = document.getElementById('btn-copy-http').dataset.url || '';
  if (!url) return;
  navigator.clipboard.writeText(url).then(() => {
    const btn = document.getElementById('btn-copy-http');
    btn.textContent = '✓';
    setTimeout(() => { btn.textContent = '🌐'; }, 1200);
  });
});

document.getElementById('btn-copy-path').addEventListener('click', () => {
  const activePath = getActivePath(state.viewer.entry, state.viewer.lang, state.viewer.layer);
  const relPath = `${state.viewer.layer}/${activePath}`;
  const fullPath = state.ui.workbenchKnowledgeRoot ? `${state.ui.workbenchKnowledgeRoot}/${relPath}` : relPath;
  navigator.clipboard.writeText(fullPath).then(() => {
    const btn = document.getElementById('btn-copy-path');
    btn.textContent = '✓';
    setTimeout(() => { btn.textContent = '📂'; }, 1200);
  });
});

document.getElementById('btn-lang-en').addEventListener('click', () => switchLang('en'));
document.getElementById('btn-lang-zh').addEventListener('click', () => switchLang('zh'));

initHighlightUI();
