export function McpChannelToolsHost() {
  return (
    <>
      <div className="settings-field">
        <label htmlFor="settings-mcp-channel">MCP channel tools</label>
        <select id="settings-mcp-channel" aria-label="MCP channel">
          <option value="workbench">/mcp/workbench</option>
          <option value="cursor_ide">/mcp/cursor_ide</option>
          <option value="mobile">/mcp/mobile</option>
        </select>
        <span className="settings-field-hint">
          Only checked tools appear in tools/list and are allowed on call_tool for that door.
          Changes apply immediately. create_note on /mcp/workbench and /mcp/cursor_ide uses a
          Host file path; on /mcp/mobile it uses Markdown content.
        </span>
      </div>
      <div id="settings-mcp-tool-groups" />
      <div className="settings-panel-actions">
        <button
          type="button"
          id="btn-settings-mcp-tools-select-all"
          className="btn-settings-save"
          aria-label="Select all tools"
        >
          All
        </button>
        <button
          type="button"
          id="btn-settings-mcp-tools-deselect-all"
          className="btn-settings-save"
          aria-label="Deselect all tools"
        >
          None
        </button>
      </div>
      <div id="settings-result-mcp-tools" className="settings-result" />
    </>
  );
}
