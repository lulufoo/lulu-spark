// @ts-nocheck — DOM wiring stays unchecked like checkJs:false.
import { initMermaid } from '../shared/mermaid-render.ts';
import { openKbDoc } from '../corpus/corpus-viewer.ts';
import { initHighlightUI } from './viewer/body.tsx';
import { enterEditMode, exitEditMode, openDoc, saveDoc, switchLang } from './viewer/doc.tsx';
import { closeModal, openCreateNote } from './viewer/create.ts';

export { openKbDoc };
export { showPendingBadge, hidePendingBadge, openCommitDialog } from './viewer/commit.tsx';
export { renderDocBody } from './viewer/body.tsx';
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
