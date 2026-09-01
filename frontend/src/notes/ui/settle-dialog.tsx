import { useSyncExternalStore } from 'react';
import {
  closeSettleDialog,
  doSettle,
  setSettleContent,
  setSettleRepo,
  setSettleSlug,
  setSettleThemeInput,
  setSettleThemeSelect,
  settleOpenStore,
} from '../commands/settle-dialog.ts';
import { settleViewStore } from '../state/settle.ts';

export { closeSettleDialog, openSettleDialog } from '../commands/settle-dialog.ts';

export function SettleDialog() {
  const open = useSyncExternalStore(settleOpenStore.subscribe, settleOpenStore.getSnapshot);
  const view = useSyncExternalStore(settleViewStore.subscribe, settleViewStore.getSnapshot);

  return (
    <div
      id="settle-dialog"
      className={open ? 'open' : undefined}
      onClick={(e) => {
        if (e.target === e.currentTarget) closeSettleDialog();
      }}
    >
      <div id="settle-dialog-box">
        <h3>⬆ Promote to knowledge repo</h3>
        <div className="settle-field">
          <label htmlFor="settle-repo-select">Target repo</label>
          <select
            id="settle-repo-select"
            value={view.repo}
            onChange={(e) => void setSettleRepo(e.target.value)}
          >
            <option value="">Choose repository…</option>
            {view.repos.map((r) => (
              <option key={r.fullName} value={r.fullName}>
                {r.description ? `${r.fullName} — ${r.description}` : r.fullName}
              </option>
            ))}
          </select>
        </div>
        <div className="settle-field">
          <label>Target folder (doc-theme)</label>
          <select
            id="settle-theme-select"
            disabled={view.dirsLoading}
            value={view.themeSelect}
            onChange={(e) => setSettleThemeSelect(e.target.value)}
          >
            {view.dirsLoading ? <option value="">Loading folders…</option> : null}
            {view.dirsError ? <option value="">Failed to load: {view.dirsError}</option> : null}
            {!view.dirsLoading && !view.dirsError ? (
              <>
                <option value=".">. (root)</option>
                {view.dirs.map((d) => (
                  <option key={d} value={d}>
                    {d}
                  </option>
                ))}
                <option value="__new__">＋ New folder…</option>
              </>
            ) : null}
          </select>
          <input
            id="settle-theme-input"
            type="text"
            placeholder="＋ New folder…"
            autoComplete="off"
            style={{ display: view.showThemeInput ? undefined : 'none' }}
            value={view.themeInput}
            onChange={(e) => setSettleThemeInput(e.target.value)}
          />
        </div>
        <div className="settle-field">
          <label>Filename</label>
          <div className="settle-filename-row">
            <span className="settle-filename-ts" id="settle-filename-ts">
              {view.filenameTs}
            </span>
            <span className="settle-filename-dash">-</span>
            <input
              id="settle-slug"
              type="text"
              placeholder="Enter name"
              autoComplete="off"
              spellCheck={false}
              value={view.slug}
              onChange={(e) => setSettleSlug(e.target.value)}
            />
            <span className="settle-filename-ext">.md</span>
          </div>
          <div id="settle-file-warn">
            {view.fileWarnPath ? (
              <span style={{ color: '#cf222e', fontSize: 11 }}>⚠ File already exists: {view.fileWarnPath}</span>
            ) : null}
          </div>
        </div>
        <div className="settle-field">
          <label>Body</label>
          <textarea
            id="settle-content"
            rows={10}
            value={view.content}
            onChange={(e) => setSettleContent(e.target.value)}
          ></textarea>
        </div>
        <div id="settle-result">
          {view.resultKind === 'ok' ? (
            <>
              <span style={{ color: '#1a7f37' }}>✓ {view.resultText}</span>
              {'　'}
              {view.resultUrl ? (
                <a href={view.resultUrl} target="_blank" style={{ fontSize: 11, wordBreak: 'break-all' }}>
                  {view.resultUrl}
                </a>
              ) : null}
              {view.resultWarns.length ? (
                <>
                  <br />
                  <span style={{ color: '#9a6700', fontSize: 11 }}>⚠ {view.resultWarns.join('；')}</span>
                </>
              ) : null}
            </>
          ) : null}
          {view.resultKind === 'err' ? (
            <span style={{ color: '#cf222e' }}>Failed: {view.resultText}</span>
          ) : null}
        </div>
        <div id="settle-dialog-actions">
          <button id="btn-settle-cancel" type="button" className="md-header-btn" onClick={() => closeSettleDialog()}>
            Cancel
          </button>
          <button
            id="btn-settle-submit"
            type="button"
            className="md-header-btn primary"
            disabled={view.submitDisabled}
            onClick={() => void doSettle()}
          >
            {view.submitLabel}
          </button>
        </div>
      </div>
    </div>
  );
}
