import { useSyncExternalStore } from 'react';
import { OverlayDismissButton } from '../../shared/overlay-dismiss-button.tsx';
import { bindOpenStore, closeBindDialog, openBindDialog } from '../commands/bind-dialog.ts';

export { closeBindDialog, openBindDialog } from '../commands/bind-dialog.ts';

export function BindDialog() {
  const open = useSyncExternalStore(bindOpenStore.subscribe, bindOpenStore.getSnapshot);
  return (
    <div
      id="bind-dialog"
      className={open ? 'open' : undefined}
      onClick={(e) => {
        if (e.target === e.currentTarget) closeBindDialog();
      }}
    >
      <div id="bind-dialog-box">
        <div id="bind-dialog-header">
          <div id="bind-dialog-heading">
            <span id="bind-dialog-title">Bind device</span>
            <p id="bind-dialog-lead">Pair this Mac with the Lulu Spark Android app on the same local network.</p>
          </div>
          <OverlayDismissButton id="btn-bind-close" onClick={() => closeBindDialog()} />
        </div>
        <div id="bind-dialog-body">
          <div id="bind-qr-card">
            <div id="bind-preview"></div>
          </div>
          <div id="bind-dialog-meta">
            <div className="bind-meta-row">
              <span className="bind-meta-label">Status</span>
              <div id="bind-status" role="status" aria-live="polite"></div>
            </div>
            <div className="bind-meta-row">
              <span className="bind-meta-label">Code</span>
              <div id="bind-countdown"></div>
            </div>
            <p id="bind-host" hidden></p>
            <ol id="bind-steps">
              <li>Open the Lulu Spark Android app.</li>
              <li>Scan this code on the same local network.</li>
              <li>Keep this window open until binding succeeds.</li>
            </ol>
          </div>
        </div>
        <div id="bind-dialog-footer">
          <p id="bind-footer-hint">A new code is available if this one expires.</p>
          <button id="btn-bind-refresh" type="button" hidden onClick={() => { void openBindDialog(); }}>
            New code
          </button>
        </div>
      </div>
    </div>
  );
}
