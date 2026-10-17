import { openBindDialog } from '../commands/bind-dialog.ts';

export { closeBindDialog, openBindDialog } from '../commands/bind-dialog.ts';

/**
 * The Devices page of Settings: pairing code for the Android app. Opening the page issues a code
 * (`openBindDialog`); leaving the page or closing Settings stops it (`closeBindDialog`).
 */
export function BindPanel() {
  return (
    <div id="settings-panel-devices" className="settings-panel">
      <div id="bind-dialog-box">
        <div id="bind-dialog-heading">
          <span id="bind-dialog-title">Pair device</span>
          <p id="bind-dialog-lead">Pair this Mac with the Lulu Spark Android app on the same local network.</p>
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
          </div>
        </div>
        <ol id="bind-steps">
          <li>Open the Lulu Spark Android app.</li>
          <li>Scan this code on the same local network.</li>
          <li>Keep this page open until binding succeeds.</li>
        </ol>
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
