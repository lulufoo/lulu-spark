const WORKSPACE_PANEL_PREFIX = 'workspace:';

/** Every `workspace:<id>` nav item shows the one workspace page. */
export function isWorkspacePanel(panelId: string): boolean {
  return panelId.startsWith(WORKSPACE_PANEL_PREFIX);
}

export function workspaceIdOfPanel(panelId: string): string {
  return panelId.slice(WORKSPACE_PANEL_PREFIX.length);
}

export function switchPanel(panelId: string) {
  const domId = isWorkspacePanel(panelId) ? 'workspace' : panelId;
  document.querySelectorAll('.settings-nav-item').forEach((btn) => {
    const el = btn as HTMLElement;
    el.classList.toggle('active', el.dataset.panel === panelId);
  });
  document.querySelectorAll('.settings-panel').forEach((panel) => {
    panel.classList.toggle('active', panel.id === `settings-panel-${domId}`);
  });
}

export function switchSettingsTab(panelId: string, tabId: string) {
  const root = document.getElementById(`settings-panel-${panelId}`);
  if (!root || !tabId) return;
  root.querySelectorAll('.settings-tab').forEach((btn) => {
    const el = btn as HTMLElement;
    el.classList.toggle('active', el.dataset.tab === tabId);
  });
  root.querySelectorAll('.settings-tab-panel').forEach((panel) => {
    const el = panel as HTMLElement;
    el.classList.toggle('active', el.dataset.tab === tabId);
  });
}
