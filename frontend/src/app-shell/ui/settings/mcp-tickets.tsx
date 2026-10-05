export function McpTicketsHost() {
  return (
    <>
      <div className="settings-field">
        <label htmlFor="settings-mcp-ticket-channel">MCP channel</label>
        <select id="settings-mcp-ticket-channel" aria-label="MCP ticket channel">
          <option value="cursor">Cursor</option>
          <option value="codex">Codex</option>
          <option value="claude">Claude</option>
          <option value="spark">Spark</option>
          <option value="mobile">Mobile</option>
        </select>
      </div>

      <div id="settings-mcp-tickets-spark" className="settings-mcp-ticket-pane" hidden>
        <div className="settings-mcp-ticket-card settings-mcp-ticket-card-masked">
          <div className="settings-mcp-ticket-kicker">Spark</div>
          <div id="settings-mcp-spark-mask" className="settings-mcp-token-mask">
            No live ticket
          </div>
          <p id="settings-mcp-spark-state" className="settings-field-hint">
            Host-issued ticket. Masked. Expire voids it for the next session.
          </p>
        </div>
        <div className="settings-panel-actions">
          <button
            type="button"
            id="btn-settings-mcp-spark-expire"
            className="btn-settings-save"
            disabled
          >
            Expire
          </button>
        </div>
      </div>

      <div id="settings-mcp-tickets-mobile" className="settings-mcp-ticket-pane" hidden>
        <p className="settings-field-hint">
          Bound devices. Tokens stay masked. Expire voids that device ticket.
        </p>
        <div id="settings-mcp-device-list" className="settings-mcp-device-list" />
      </div>

      <div id="settings-mcp-tickets-ide" className="settings-mcp-ticket-pane">
        <div className="settings-field">
          <label htmlFor="settings-mcp-server-block" id="settings-mcp-server-block-label">
            Cursor server block
          </label>
          <textarea
            id="settings-mcp-server-block"
            rows={8}
            spellCheck={false}
            autoComplete="off"
            readOnly
          />
          <span className="settings-field-hint">
            No ticket: Generate. Live ticket: Refresh replaces it. Copy pastes the current block.
            Host does not write that file.
          </span>
        </div>
        <div className="settings-panel-actions">
          <button
            type="button"
            id="btn-settings-mcp-primary"
            className="btn-settings-save"
            data-mcp-ticket-action="generate"
          >
            Generate
          </button>
          <button type="button" id="btn-settings-mcp-copy" className="btn-settings-save" disabled>
            Copy
          </button>
        </div>
      </div>

      <div id="settings-result-mcp" className="settings-result" />
    </>
  );
}
