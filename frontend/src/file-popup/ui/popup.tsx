import { useLayoutEffect, useSyncExternalStore, type CSSProperties, type MouseEvent } from 'react';
import { createRoot, type Root } from 'react-dom/client';
import { flushSync } from 'react-dom';
import { renderDocMarkdown, setDocEditMode } from '../../doc-editor/view.tsx';
import {
  cancelFilePopupEdit,
  closeFilePopup,
  enterFilePopupEdit,
  saveFilePopup,
} from '../commands/popup.ts';
import { viewStore } from '../state/store.ts';

const overlayStyle: CSSProperties = {
  position: 'fixed',
  inset: 0,
  zIndex: 2100,
  display: 'flex',
  alignItems: 'center',
  justifyContent: 'center',
  background: 'rgba(0, 0, 0, 0.45)',
};

const boxStyle: CSSProperties = {
  background: '#fff',
  border: '1px solid #d0d7de',
  borderRadius: 14,
  width: 'min(860px, 94vw)',
  height: 'min(80vh, 720px)',
  boxShadow: '0 16px 48px rgba(31, 35, 40, 0.18)',
  display: 'flex',
  flexDirection: 'column',
  overflow: 'hidden',
};

const headerStyle: CSSProperties = {
  padding: '14px 18px',
  borderBottom: '1px solid #d8dee4',
  display: 'flex',
  alignItems: 'center',
  gap: 12,
};

const titleStyle: CSSProperties = {
  margin: 0,
  fontSize: 16,
  fontWeight: 600,
  color: '#1f2328',
  flex: 1,
  overflow: 'hidden',
  textOverflow: 'ellipsis',
  whiteSpace: 'nowrap',
};

const actionsStyle: CSSProperties = {
  display: 'flex',
  gap: 8,
  flexShrink: 0,
};

const bodyWrapStyle: CSSProperties = {
  flex: 1,
  minHeight: 0,
  position: 'relative',
};

const surfaceStyle: CSSProperties = {
  position: 'absolute',
  inset: 0,
  overflow: 'auto',
  padding: 18,
};

const errorStyle: CSSProperties = {
  margin: 0,
  padding: '8px 18px',
  color: '#b42318',
  fontSize: 13,
};

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

function asTextArea(el: HTMLElement | null) {
  if (!el || !('value' in el)) return null;
  return el as HTMLTextAreaElement;
}

function paintDoc(editing: boolean, content: string) {
  const bodyEl = document.getElementById('file-popup-body');
  const editAreaEl = asTextArea(document.getElementById('file-popup-edit-area'));
  setDocEditMode({ bodyEl, editAreaEl, text: content, editing });
  if (!editing) renderDocMarkdown(bodyEl, content);
}

export function FilePopup() {
  const view = useSyncExternalStore(viewStore.subscribe, viewStore.getSnapshot);

  useLayoutEffect(() => {
    if (!view.open) return;
    paintDoc(view.editing, view.content);
  }, [view.open, view.editing, view.content, view.loading]);

  useLayoutEffect(() => {
    if (!view.open) return;
    function onKey(event: KeyboardEvent) {
      if (event.key === 'Escape' && !viewStore.getSnapshot().saving) closeFilePopup();
    }
    document.addEventListener('keydown', onKey);
    return () => document.removeEventListener('keydown', onKey);
  }, [view.open]);

  if (!view.open) return null;

  function onOverlayClick(event: MouseEvent<HTMLDivElement>) {
    if (event.target === event.currentTarget && !view.saving) closeFilePopup();
  }

  const busy = view.loading || view.saving;

  return (
    <div id="file-popup" role="dialog" aria-modal="true" aria-labelledby="file-popup-title" style={overlayStyle} onClick={onOverlayClick}>
      <div style={boxStyle}>
        <header style={headerStyle}>
          <h3 id="file-popup-title" style={titleStyle}>
            {view.title}
          </h3>
          <div style={actionsStyle}>
            {view.editing ? (
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
            ) : (
              <button type="button" className="md-header-btn primary" disabled={busy || Boolean(view.error)} onClick={() => enterFilePopupEdit()}>
                Edit
              </button>
            )}
            <button type="button" className="md-header-btn" disabled={view.saving} onClick={() => closeFilePopup()}>
              Close
            </button>
          </div>
        </header>
        {view.error ? (
          <p style={errorStyle} role="alert">
            {view.error}
          </p>
        ) : null}
        <div style={bodyWrapStyle}>
          {view.loading ? (
            <p style={{ ...surfaceStyle, color: '#656d76' }}>Loading…</p>
          ) : (
            <>
              <div id="file-popup-body" style={surfaceStyle} />
              <textarea
                id="file-popup-edit-area"
                style={{ ...surfaceStyle, display: 'none', resize: 'none', border: 0, fontFamily: 'ui-monospace, SFMono-Regular, Menlo, monospace', fontSize: 13, lineHeight: 1.5 }}
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
