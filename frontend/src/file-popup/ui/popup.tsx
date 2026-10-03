import { useLayoutEffect, useState, useSyncExternalStore, type MouseEvent } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { flushSync } from 'react-dom';
import { OverlayDismissButton } from '../../shared/overlay-dismiss-button.tsx';
import {
  cancelFilePopupEdit,
  closeFilePopup,
  copyFilePopupPath,
  enterFilePopupEdit,
  saveFilePopup,
} from '../commands/popup.ts';
import { viewStore } from '../state/store.ts';
import { paintFilePopupDoc } from './paint.ts';

const COPY_LABEL = 'Copy';
const COPY_FLASH_MS = 1200;

let hostRoot: Root | null = null;

export function ensureFilePopupHost() {
  if (typeof document === 'undefined') return;
  if (document.getElementById('file-popup')) return;
  let host = document.getElementById('file-popup-root');
  if (!host) {
    host = document.createElement('div');
    host.id = 'file-popup-root';
    document.body.appendChild(host);
  }
  if (!hostRoot) hostRoot = createRoot(host);
  flushSync(() => {
    hostRoot?.render(<FilePopup />);
  });
}

export function FilePopup() {
  const view = useSyncExternalStore(viewStore.subscribe, viewStore.getSnapshot);
  const [copyLabel, setCopyLabel] = useState(COPY_LABEL);

  useLayoutEffect(() => {
    if (!view.open) return;
    void paintFilePopupDoc({
      editing: view.editing,
      content: view.content,
      identityKey: view.identityKey,
    });
  }, [view.open, view.editing, view.content, view.loading, view.path, view.identityKey]);

  useLayoutEffect(() => {
    if (!view.open) return;
    function onKey(event: KeyboardEvent) {
      if (event.key === 'Escape' && !viewStore.getSnapshot().saving) closeFilePopup();
    }
    document.addEventListener('keydown', onKey);
    return () => document.removeEventListener('keydown', onKey);
  }, [view.open]);

  useLayoutEffect(() => {
    setCopyLabel(COPY_LABEL);
  }, [view.path, view.open]);

  if (!view.open) return null;

  function onOverlayClick(event: MouseEvent<HTMLDivElement>) {
    if (event.target === event.currentTarget && !view.saving) closeFilePopup();
  }

  const busy = view.loading || view.saving;
  const canEdit = Boolean(view.path);
  const canCopy = Boolean(view.path);

  function onCopyPath() {
    void copyFilePopupPath().then((ok) => {
      if (!ok) return;
      setCopyLabel('✓');
      setTimeout(() => setCopyLabel(COPY_LABEL), COPY_FLASH_MS);
    });
  }

  return (
    <div id="file-popup" role="dialog" aria-modal="true" aria-labelledby="file-popup-title" onClick={onOverlayClick}>
      <div className="file-popup-box">
        <div className="viewer-header">
          <h3 id="file-popup-title" className="viewer-panel-title">
            {view.title}
          </h3>
          <div className="file-popup-actions">
            {canEdit && view.editing ? (
              <>
                <button type="button" className="md-header-btn" disabled={busy} onClick={() => cancelFilePopupEdit()}>
                  Cancel
                </button>
                <button
                  type="button"
                  className="md-header-btn primary"
                  disabled={busy}
                  onClick={() => {
                    void saveFilePopup();
                  }}
                >
                  {view.saving ? 'Saving…' : 'Save'}
                </button>
              </>
            ) : canEdit ? (
              <button type="button" className="md-header-btn primary" disabled={busy || Boolean(view.error)} onClick={() => enterFilePopupEdit()}>
                Edit
              </button>
            ) : null}
            <button
              type="button"
              className="md-header-btn"
              data-role="copy-file-path"
              title="Copy absolute path"
              disabled={!canCopy}
              onClick={onCopyPath}
            >
              {copyLabel}
            </button>
            <OverlayDismissButton disabled={view.saving} onClick={() => closeFilePopup()} />
          </div>
        </div>
        {view.error ? (
          <p className="file-popup-error" role="alert">
            {view.error}
          </p>
        ) : null}
        <div className="viewer-content-row">
          {view.loading ? (
            <p className="file-popup-loading">Loading…</p>
          ) : (
            <>
              <div id="file-popup-body" className="viewer-body" />
              <textarea
                id="file-popup-edit-area"
                className="viewer-edit-area"
                style={{ display: 'none' }}
                spellCheck={false}
                disabled={busy}
                defaultValue={view.content}
              />
            </>
          )}
        </div>
      </div>
    </div>
  );
}
