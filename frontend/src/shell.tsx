import { BindDialog } from './app-shell/ui/bind-dialog.tsx';
import { SettingsDialog } from './app-shell/ui/settings/dialog.tsx';
import { SettingsDialogChrome } from './app-shell/ui/settings/chrome.tsx';

void SettingsDialogChrome;
import { KbCommentDialog } from './knowledge/ui/comments.tsx';
import { NoteCommentDialog } from './notes/ui/comments.tsx';
import { DeleteDialog } from './notes/ui/delete-dialog.tsx';
import { MoveProjectDialog } from './notes/ui/move-project-dialog.tsx';
import { SettleDialog } from './notes/ui/settle-dialog.tsx';
import { ReadLaterDialog } from './read-later/ui/dialog.tsx';
import { CommentDeleteDialog } from './shared/comment-delete.tsx';

/** Desktop chrome as React nodes. Dialogs own their open state. */
export function Shell() {
  return (
    <>
      {/* <!-- Bind device dialog --> */}
      <BindDialog />

      {/* <!-- KB Comment dialog --> */}
      <KbCommentDialog />

      {/* <!-- Floating highlight button --> */}
      <button id="highlight-add-btn" className="viewer-highlight-btn" style={{ display: 'none', position: 'fixed', zIndex: 9999 }}>Highlight</button>
      {/* <!-- KB Floating highlight button --> */}
      <button id="kb-highlight-add-btn" className="viewer-highlight-btn" style={{ display: 'none', position: 'fixed', zIndex: 9999 }}>Highlight</button>
      {/* <!-- Comment preview tooltip --> */}
      <div id="comment-preview-tip"></div>

      {/* <!-- Comment dialog --> */}
      <NoteCommentDialog />

      {/* <!-- Move project dialog --> */}
      <MoveProjectDialog />

      {/* <!-- Settle dialog --> */}
      <SettleDialog />

      {/* <!-- Comment delete confirm --> */}
      <CommentDeleteDialog />

      {/* <!-- Delete confirm --> */}
      <DeleteDialog />

      {/* <!-- Settings dialog --> */}
      <SettingsDialog />

      {/* <!-- Read Later dialog --> */}
      <ReadLaterDialog />
    </>
  );
}
