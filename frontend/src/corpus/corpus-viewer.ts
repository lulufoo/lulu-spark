// @ts-nocheck — DOM wiring stays unchecked like checkJs:false.
import { closeKbCommitDialog, doKbCommit, openKbCommitDialog, _kbRevertAll } from './corpus-viewer/commit.tsx';
import { closeKbModal, saveKbDoc, _kbEnterEditMode, _kbExitEditMode } from './corpus-viewer/doc.tsx';

export { mountKbReader } from './corpus-viewer/mount.tsx';
export { openKbDoc, saveKbDoc, closeKbModal } from './corpus-viewer/doc.tsx';
export { showKbReindexBtn } from './corpus-viewer/chrome.tsx';

document.getElementById('kb-md-close')?.addEventListener('click', closeKbModal);
document.getElementById('kb-md-backdrop')?.addEventListener('click', closeKbModal);

document.addEventListener('keydown', e => {
  if (e.key !== 'Escape') return;
  const kbModal = document.getElementById('kb-md-modal');
  if (!kbModal || kbModal.style.display === 'none') return;
  const kbCommitDialog = document.getElementById('kb-commit-dialog');
  if (kbCommitDialog && kbCommitDialog.classList.contains('open')) return;
  const kbDialog = document.getElementById('kb-comment-dialog');
  if (kbDialog && kbDialog.style.display !== 'none') return;
  closeKbModal();
});

document.getElementById('kb-btn-edit')?.addEventListener('click', _kbEnterEditMode);
document.getElementById('kb-btn-save')?.addEventListener('click', saveKbDoc);
document.getElementById('kb-btn-cancel-edit')?.addEventListener('click', () => {
  _kbExitEditMode();
});

document.getElementById('kb-btn-pending')?.addEventListener('click', openKbCommitDialog);
document.getElementById('kb-btn-commit-cancel')?.addEventListener('click', closeKbCommitDialog);
document.getElementById('kb-btn-commit-ok')?.addEventListener('click', doKbCommit);
document.getElementById('kb-btn-revert-all')?.addEventListener('click', e => _kbRevertAll(e.currentTarget));
document.getElementById('kb-commit-dialog')?.addEventListener('click', e => {
  if (e.target === document.getElementById('kb-commit-dialog')) closeKbCommitDialog();
});
document.getElementById('kb-commit-msg')?.addEventListener('keydown', e => {
  if (e.key === 'Enter') { e.preventDefault(); doKbCommit(); }
  if (e.key === 'Escape') { e.stopPropagation(); closeKbCommitDialog(); }
});

document.getElementById('kb-btn-copy-http')?.addEventListener('click', e => {
  const btn = e.currentTarget;
  const url = btn.dataset.url || '';
  if (!url) return;
  navigator.clipboard.writeText(url).then(() => {
    const orig = btn.textContent;
    btn.textContent = '✓';
    setTimeout(() => { btn.textContent = orig; }, 1200);
  }).catch(() => {});
});

document.getElementById('kb-btn-copy-path')?.addEventListener('click', e => {
  const btn = e.currentTarget;
  const path = btn.dataset.path || '';
  if (!path) return;
  navigator.clipboard.writeText(path).then(() => {
    const orig = btn.textContent;
    btn.textContent = '✓';
    setTimeout(() => { btn.textContent = orig; }, 1200);
  }).catch(() => {});
});
