// @ts-nocheck — chrome port; leftover islands still find these IDs.
import { BindDialog, openBindDialog } from './app-shell/bind-dialog.tsx';
import { CommitChangesDialog } from './app-shell/commit-dialog.tsx';
import { ConvertDialog, openConvertDialog } from './app-shell/convert-dialog.tsx';
import { MoveDocDialog, openMoveDocDialog } from './app-shell/move-dialog.tsx';
import { SettingsDialog, openSettingsDialog } from './app-shell/settings-dialog.tsx';
import { SettingsDialogChrome } from './app-shell/settings/chrome.tsx';
import { SkillsDialog, _openSkillsDialog } from './app-shell/skills-dialog.tsx';
import { KbCommentDialog } from './corpus/corpus-comments.tsx';
import { KbDiffDialog } from './corpus/corpus-diff-dialog.tsx';
import { KbCommitDialog } from './corpus/corpus-viewer/commit.tsx';
import { NoteCommentDialog } from './notes/comments.tsx';
import { DeleteDialog } from './notes/delete-dialog.tsx';
import { MoveProjectDialog } from './notes/move-project-dialog.tsx';
import { SettleDialog } from './notes/settle-dialog.tsx';
import { MdCommitDialog } from './notes/viewer/commit.tsx';
import { ReadLaterDialog } from './read-later/dialog.tsx';
import { CommentDeleteDialog } from './shared/comment-delete.tsx';
import { TodoTaskDialog } from './todo-task/dialog.tsx';

function closeMenuDropdowns() {
  document.getElementById('sync-menu-dropdown')?.classList.remove('open');
  document.getElementById('tools-menu-dropdown')?.classList.remove('open');
  document.getElementById('skills-menu-dropdown')?.classList.remove('open');
}

/** Desktop chrome as React nodes. Dialogs own their open state. */
export function Shell() {
  return (
    <>

      <header>
        <h1>
          <a id="btn-nav-home-title" href="#/home" className="header-home-link">LuLu Workbench</a>
          <a id="btn-nav-home" href="#/home" className="header-nav-back" hidden>← Home</a>
        </h1>
        <div id="gs-wb-wrap" className="gs-search-wrap" hidden>
          <input id="gs-wb-input" className="gs-search-input" type="text" placeholder="Search notes…" autoComplete="off" spellCheck={false} />
          <button id="gs-wb-rebuild-btn" className="gs-rebuild-btn" style={{ display: 'none' }} title="Rebuild Workbench index">↺</button>
          <div id="gs-wb-dropdown" className="gs-search-dropdown" style={{ display: 'none' }}></div>
        </div>
        <div id="gs-kb-wrap" className="gs-search-wrap" hidden>
          <input id="gs-kb-input" className="gs-search-input" type="text" placeholder="Search knowledge…" autoComplete="off" spellCheck={false} />
          <button id="gs-kb-rebuild-btn" className="gs-rebuild-btn" style={{ display: 'none' }} title="Rebuild knowledge index">↺</button>
          <div id="gs-kb-dropdown" className="gs-search-dropdown" style={{ display: 'none' }}></div>
        </div>
        <a href="https://github.com/lulufoo/lulu-workbench" target="_blank">GitHub ↗</a>
        <div id="sync-menu-wrap">
          <button id="btn-sync-menu">⇕ Sync</button>
          <div id="sync-menu-dropdown">
            <button id="btn-push-index">↑ Commit changes</button>
            <button id="btn-pull">↓ Update project</button>
            <button id="btn-local-refresh">⟳ Refresh local</button>
          </div>
        </div>
        <div id="tools-menu-wrap">
          <button id="btn-tools-menu">⛓ Tools</button>
          <div id="tools-menu-dropdown">
            <button
              id="btn-move-doc-header"
              type="button"
              onClick={() => {
                closeMenuDropdowns();
                openMoveDocDialog();
              }}
            >
              <span className="tools-menu-icon">↗</span>GitHub
            </button>
            <button
              id="btn-convert"
              type="button"
              onClick={() => {
                closeMenuDropdowns();
                openConvertDialog('base64');
              }}
            >
              <span className="tools-menu-icon">🔀</span>Convert
            </button>
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
        <div id="skills-menu-wrap">
          <button id="btn-skills-menu">✦ SKILL</button>
          <div id="skills-menu-dropdown">
            <button
              id="btn-skill-workbench"
              type="button"
              onClick={() => {
                closeMenuDropdowns();
                _openSkillsDialog();
              }}
            >
              ✦ Lulu Workbench Skills
            </button>
          </div>
        </div>
      </header>

      {/* <!-- Convert tool dialog (Base64 + QR) --> */}
      <ConvertDialog />

      {/* <!-- Bind device dialog --> */}
      <BindDialog />

      {/* <!-- Workbench Commit dialog --> */}
      <MdCommitDialog />

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

      {/* <!-- Commit changes dialog --> */}
      <CommitChangesDialog />

      {/* <!-- KB Diff dialog --> */}
      <KbDiffDialog />

      {/* <!-- Move project dialog --> */}
      <MoveProjectDialog />

      {/* <!-- Move document dialog --> */}
      <MoveDocDialog />

      {/* <!-- Settle dialog --> */}
      <SettleDialog />

      {/* <!-- Comment delete confirm --> */}
      <CommentDeleteDialog />

      {/* <!-- Delete confirm --> */}
      <DeleteDialog />

      {/* <!-- Settings dialog --> */}
      <SettingsDialog />

      {/* <!-- Plan Task dialog --> */}
      <TodoTaskDialog />

      {/* <!-- Read Later dialog --> */}
      <ReadLaterDialog />

      {/* <!-- Skills dialog --> */}
      <SkillsDialog />

    </>
  );
}
