export function emitKnowledgeTab(tabId: string) {
  window.dispatchEvent(new CustomEvent('settings-knowledge-tab', { detail: tabId }));
}

export function switchPanel(panelId: string) {
  document.querySelectorAll('.settings-nav-item').forEach((btn) => {
    const el = btn as HTMLElement;
    el.classList.toggle('active', el.dataset.panel === panelId);
  });
  document.querySelectorAll('.settings-panel').forEach((panel) => {
    panel.classList.toggle('active', panel.id === `settings-panel-${panelId}`);
  });
  if (panelId === 'knowledge') {
    const active = document.querySelector(
      '#settings-panel-knowledge .settings-tab.active',
    ) as HTMLElement | null;
    emitKnowledgeTab(active?.dataset.tab || 'directory');
  }
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
  if (
    panelId === 'knowledge' &&
    document.getElementById('settings-panel-knowledge')?.classList.contains('active')
  ) {
    emitKnowledgeTab(tabId);
  }
}
