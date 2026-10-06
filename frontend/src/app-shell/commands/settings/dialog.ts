import { loadKbHidePatterns } from './kb-hide-patterns.ts';
import { applyEngineCategorySelection, saveAssistantEnginePanel } from './engine.ts';
import {
  deselectAllMcpChannelTools,
  loadMcpChannelTools,
  paintMcpToolGroups,
  persistMcpChannelTools,
  selectAllMcpChannelTools,
} from './mcp.ts';
import {
  clearMcpServerBlock,
  copyCursorIdeServerBlock,
  expireMobileDevice,
  expireSparkTicket,
  loadMcpTicketView,
  runCursorIdePrimaryAction,
} from './mcp-tickets.ts';
import {
  setResult,
} from '../../state/settings/store.ts';
import { settingsOpenStore } from '../../state/dialog-open.ts';
import {
  clearNotesCategoryError,
  closeNotesCategoryEditor,
  loadNotesCategories,
} from './notes-categories.ts';
import { loadSettingsSnapshot } from './snapshot.ts';
import { switchPanel, switchSettingsTab } from '../../ui/settings/tabs.ts';
import { mountSettingsListSelects } from '../../ui/settings/list-select.ts';

export { settingsOpenStore };

type SettingsOpenOpts = { panel?: string; tab?: string };

function input(id: string): HTMLInputElement {
  return document.getElementById(id) as HTMLInputElement;
}

function btn(id: string): HTMLButtonElement {
  return document.getElementById(id) as HTMLButtonElement;
}

let wired = false;

export function ensureWired() {
  if (wired) return;
  // renderToHtml paints into a detached box. Wiring against document there
  // throws and React drops the Settings subtree from the HTML snapshot.
  if (!document.getElementById('settings-dialog-box')) return;
  wireSettingsDialog();
  wired = true;
}

export function closeSettingsDialog() {
  closeNotesCategoryEditor();
  settingsOpenStore.set(false);
  document.getElementById('settings-dialog')?.classList.remove('open');
}

export async function openSettingsDialog(opts: SettingsOpenOpts = {}) {
  closeNotesCategoryEditor();
  settingsOpenStore.set(true);
  document.getElementById('settings-dialog')?.classList.add('open');
  ensureWired();
  setResult('settings-result-knowledge', '');
  setResult('settings-result-llm', '');
  setResult('settings-result-mcp', '');
  setResult('settings-result-mcp-tools', '');
  clearMcpServerBlock();
  const keyInput = input('settings-llm-api-key');
  if (keyInput) keyInput.value = '';
  const panelId = opts.panel || 'notes';
  switchSettingsTab('notes', panelId === 'notes' ? (opts.tab || 'directory') : 'directory');
  switchSettingsTab('knowledge', panelId === 'knowledge' ? (opts.tab || 'directory') : 'directory');
  switchSettingsTab('llm', 'engine');
  switchSettingsTab('mcp', panelId === 'mcp' ? (opts.tab || 'tickets') : 'tickets');
  switchPanel(panelId);
  await loadSettingsSnapshot();
  await loadKbHidePatterns();
  await loadNotesCategories();
  await loadMcpChannelTools();
  requestAnimationFrame(() => paintMcpToolGroups());
  await loadMcpTicketView();
}

function wireSettingsDialog() {
  document.querySelectorAll('.settings-tab').forEach((tabBtn) => {
    tabBtn.addEventListener('click', () => {
      const panel = tabBtn.closest('.settings-panel');
      const panelId = panel?.id?.replace(/^settings-panel-/, '');
      const tab = (tabBtn as HTMLElement).dataset.tab;
      if (panelId !== 'notes' || tab !== 'edit') closeNotesCategoryEditor();
      if (panelId === 'notes') clearNotesCategoryError();
      if (panelId && tab) switchSettingsTab(panelId, tab);
      if (panelId === 'mcp' && tab === 'tools') void loadMcpChannelTools();
    });
  });

  document.querySelectorAll('.settings-nav-item').forEach((navBtn) => {
    navBtn.addEventListener('click', async () => {
      const panelId = (navBtn as HTMLElement).dataset.panel;
      if (!panelId) return;
      if (panelId !== 'notes') closeNotesCategoryEditor();
      switchPanel(panelId);
    });
  });

  btn('btn-settings-close')?.addEventListener('click', closeSettingsDialog);
  document.getElementById('settings-dialog')?.addEventListener('click', (e) => {
    if (e.target === document.getElementById('settings-dialog')) closeSettingsDialog();
  });
  document.getElementById('settings-llm-engine')?.addEventListener('change', (e) => {
    void applyEngineCategorySelection((e.target as HTMLSelectElement).value, { clearCredential: true });
  });

  btn('btn-settings-save-llm')?.addEventListener('click', () => {
    void saveAssistantEnginePanel();
  });

  document.getElementById('btn-settings-mcp-primary')?.addEventListener('click', () => {
    void runCursorIdePrimaryAction();
  });
  document.getElementById('btn-settings-mcp-copy')?.addEventListener('click', () => {
    void copyCursorIdeServerBlock();
  });
  document.getElementById('btn-settings-mcp-spark-expire')?.addEventListener('click', () => {
    void expireSparkTicket();
  });
  document.getElementById('settings-mcp-device-list')?.addEventListener('click', (event) => {
    const target = event.target as HTMLElement | null;
    const deviceId = target?.closest('button')?.dataset.mcpDeviceExpire;
    if (deviceId) void expireMobileDevice(deviceId);
  });
  document.getElementById('settings-mcp-ticket-channel')?.addEventListener('change', () => {
    void loadMcpTicketView();
  });
  document.getElementById('settings-mcp-channel')?.addEventListener('change', () => {
    paintMcpToolGroups();
  });
  document.getElementById('settings-mcp-tool-groups')?.addEventListener('change', (event) => {
    const target = event.target as HTMLElement | null;
    if (target?.matches?.('input[data-mcp-tool]')) void persistMcpChannelTools();
  });
  document.getElementById('btn-settings-mcp-tools-select-all')?.addEventListener('click', () => {
    void selectAllMcpChannelTools();
  });
  document.getElementById('btn-settings-mcp-tools-deselect-all')?.addEventListener('click', () => {
    void deselectAllMcpChannelTools();
  });
  mountSettingsListSelects();
}
