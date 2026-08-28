import { initMermaid } from '../shared/mermaid-render.ts';
import { openKbDoc } from '../corpus/viewer.ts';
import { initHighlightUI } from './ui/viewer/body.tsx';
import { enterEditMode, exitEditMode, openDoc, saveDoc, switchLang } from './commands/viewer/doc.ts';
import { closeModal, openCreateNote } from './commands/viewer/create.ts';

export { openKbDoc };
export { showPendingBadge, hidePendingBadge, openCommitDialog } from './ui/viewer/commit.tsx';
export { renderDocBody } from './ui/viewer/body.tsx';
export { enterEditMode, exitEditMode, openDoc, saveDoc, switchLang };
export { closeModal, openCreateNote };

initMermaid();

document.addEventListener('keydown', e => {
  if (e.key === 'Escape') {
    if (document.getElementById('comment-dialog')?.classList.contains('open')) return;
    if (document.getElementById('md-commit-dialog')?.classList.contains('open')) return;
    void closeModal();
  }
});

initHighlightUI();
