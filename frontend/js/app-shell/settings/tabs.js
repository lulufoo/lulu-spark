export function emitKnowledgeTab(tabId) {
  window.dispatchEvent(new CustomEvent('settings-knowledge-tab', { detail: tabId }));
}

export function switchPanel(panelId) {
  document.querySelectorAll('.settings-nav-item').forEach(btn => {
    btn.classList.toggle('active', btn.dataset.panel === panelId);
  });
  document.querySelectorAll('.settings-panel').forEach(panel => {
    panel.classList.toggle('active', panel.id === `settings-panel-${panelId}`);
  });
  if (panelId === 'knowledge') {
    const active = document.querySelector('#settings-panel-knowledge .settings-tab.active');
    emitKnowledgeTab(active?.dataset.tab || 'directory');
  }
}

export function switchSettingsTab(panelId, tabId) {
  const root = document.getElementById(`settings-panel-${panelId}`);
  if (!root || !tabId) return;
  root.querySelectorAll('.settings-tab').forEach((btn) => {
    btn.classList.toggle('active', btn.dataset.tab === tabId);
  });
  root.querySelectorAll('.settings-tab-panel').forEach((panel) => {
    panel.classList.toggle('active', panel.dataset.tab === tabId);
  });
  if (
    panelId === 'knowledge' &&
    document.getElementById('settings-panel-knowledge')?.classList.contains('active')
  ) {
    emitKnowledgeTab(tabId);
  }
}
