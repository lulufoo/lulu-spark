export { mountKbReader } from './commands/viewer/mount.ts';
export { ReaderShell } from './ui/viewer/shell.tsx';
export { openKbDoc, saveKbDoc, closeKbModal, _kbEnterEditMode, _kbExitEditMode } from './commands/viewer/doc.ts';
export { closeKbCommitDialog, doKbCommit, openKbCommitDialog, _kbRevertAll } from './ui/viewer/commit.tsx';

import { closeKbModal } from './commands/viewer/doc.ts';

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
