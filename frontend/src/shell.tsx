import { BindDialog, openBindDialog } from './app-shell/ui/bind-dialog.tsx';
import { SparkCommitDialog } from './app-shell/ui/spark-commit-dialog.tsx';
import { SettingsDialog, openSettingsDialog } from './app-shell/ui/settings/dialog.tsx';
import { SettingsDialogChrome } from './app-shell/ui/settings/chrome.tsx';

void SettingsDialogChrome;
import { KbCommentDialog } from './knowledge/ui/comments.tsx';
import { KnowledgeDiffDialog } from './knowledge/ui/knowledge-diff-dialog.tsx';
import { KbCommitDialog } from './knowledge/ui/viewer/commit.tsx';
import { NoteCommentDialog } from './notes/ui/comments.tsx';
import { DeleteDialog } from './notes/ui/delete-dialog.tsx';
import { MoveProjectDialog } from './notes/ui/move-project-dialog.tsx';
import { SettleDialog } from './notes/ui/settle-dialog.tsx';
import { ReadLaterDialog } from './read-later/ui/dialog.tsx';
import { CommentDeleteDialog } from './shared/comment-delete.tsx';
import { SparkSearch } from './notes/ui/search.tsx';
import { KnowledgeSearch } from './knowledge/ui/search.tsx';

function closeMenuDropdowns() {
  document.getElementById('sync-menu-dropdown')?.classList.remove('open');
  document.getElementById('tools-menu-dropdown')?.classList.remove('open');
}

/** Desktop chrome as React nodes. Dialogs own their open state. */
export function Shell() {
  return (
    <>

      <header>
        <h1>
          <a id="btn-nav-home-title" href="#/home" className="header-home-link" aria-label="Lulu Spark">
            <svg className="header-home-mark" viewBox="8 8 16 16" aria-hidden="true" focusable="false">
              <path d="M8 8h7v9h9v7H8z" fill="#1d1d1f" />
              <rect x="18" y="8" width="6" height="6" rx="1.5" fill="#0071e3" />
            </svg>
          </a>
        </h1>
        <SparkSearch />
        <KnowledgeSearch />
        <div id="sync-menu-wrap">
          <button id="btn-sync-menu">⇕ Sync</button>
          <div id="sync-menu-dropdown">
            <button id="btn-push-index">↑ Commit changes</button>
            <button id="btn-pull">↓ Update project</button>
            <button id="btn-local-refresh">⟳ Refresh local</button>
          </div>
        </div>
        <div id="tools-menu-wrap">
          <button id="btn-tools-menu">⇔ Bind</button>
          <div id="tools-menu-dropdown">
            <button
              id="btn-bind"
              type="button"
              onClick={() => {
                closeMenuDropdowns();
                openBindDialog();
              }}
            >
              <span className="tools-menu-icon">📲</span>Bind device
            </button>
          </div>
        </div>
        <button
          id="btn-settings"
          type="button"
          title="Settings"
          onClick={() => {
            closeMenuDropdowns();
            void openSettingsDialog();
          }}
        >
          ⚙ Settings
        </button>
      </header>

      {/* <!-- Bind device dialog --> */}
      <BindDialog />

      {/* <!-- KB Commit dialog --> */}
      <KbCommitDialog />

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

      {/* <!-- Spark commit dialog --> */}
      <SparkCommitDialog />

      {/* <!-- Knowledge Diff dialog --> */}
      <KnowledgeDiffDialog />

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
