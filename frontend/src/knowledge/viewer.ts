export { mountKbReader } from './commands/viewer/mount.ts';
export { ReaderShell } from './ui/viewer/shell.tsx';
export { openKbDoc, saveKbDoc, closeKbModal, _kbEnterEditMode, _kbExitEditMode } from './commands/viewer/doc.ts';

import { closeKbModal } from './commands/viewer/doc.ts';

document.addEventListener('keydown', e => {
  if (e.key !== 'Escape') return;
  const kbModal = document.getElementById('kb-md-modal');
  if (!kbModal || kbModal.style.display === 'none') return;
  const kbDialog = document.getElementById('kb-comment-dialog');
  if (kbDialog && kbDialog.style.display !== 'none') return;
  closeKbModal();
});
