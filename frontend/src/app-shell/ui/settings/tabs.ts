export function switchPanel(panelId: string) {
  document.querySelectorAll('.settings-nav-item').forEach((btn) => {
    const el = btn as HTMLElement;
    el.classList.toggle('active', el.dataset.panel === panelId);
  });
  document.querySelectorAll('.settings-panel').forEach((panel) => {
    panel.classList.toggle('active', panel.id === `settings-panel-${panelId}`);
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
