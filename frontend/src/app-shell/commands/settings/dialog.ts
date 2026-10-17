import {
  applyEngineCategorySelection,
  clearSavedCredentialPlaceholder,
  restoreSavedCredentialPlaceholder,
  saveAssistantEnginePanel,
} from './engine.ts';
import { loadDocCategories } from './document-categories.ts';
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
  closeNotesCategoryEditor,
  selectProjectsWorkspace,
} from './notes-categories.ts';
import { selectHidePatternsWorkspace } from './hide-patterns.ts';
import { closeWorkspaceEditor, loadDisabledWorkspaces } from './workspaces.ts';
import { loadSettingsSnapshot } from './snapshot.ts';
import { closeBindDialog, openBindDialog } from '../bind-dialog.ts';
import { saveThemeSelection } from './theme.ts';
import {
  isWorkspacePanel,
  switchPanel,
  switchSettingsTab,
  workspaceIdOfPanel,
} from '../../ui/settings/tabs.ts';
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

/** Dialogs of the Workspaces and workspace pages never outlive their page. */
function closeEditors() {
  closeNotesCategoryEditor();
  closeWorkspaceEditor();
}

export function closeSettingsDialog() {
  closeEditors();
  closeBindDialog();
  settingsOpenStore.set(false);
  document.getElementById('settings-dialog')?.classList.remove('open');
}

/**
 * Show one settings panel, always on its Directory tab. A `workspace:<id>` panel also loads that
 * workspace's projects and hidden-file rules.
 */
function showPanel(panelId: string): Promise<void> {
  closeEditors();
  switchPanel(panelId);
  // The Devices page issues a pairing code when it is shown and stops it when another page is.
  if (panelId === 'devices') void openBindDialog();
  else closeBindDialog();
  if (isWorkspacePanel(panelId)) {
    switchSettingsTab('workspace', 'directory');
    const workspaceId = workspaceIdOfPanel(panelId);
    return Promise.all([
      selectProjectsWorkspace(workspaceId),
      selectHidePatternsWorkspace(workspaceId),
    ]).then(() => undefined);
  }
  if (panelId === 'notes') switchSettingsTab('notes', 'directory');
  if (panelId === 'document') return loadDocCategories();
  return Promise.resolve();
}

export async function openSettingsDialog(opts: SettingsOpenOpts = {}) {
  closeEditors();
  settingsOpenStore.set(true);
  document.getElementById('settings-dialog')?.classList.add('open');
  ensureWired();
  setResult('settings-result-llm', '');
  setResult('settings-result-mcp', '');
  setResult('settings-result-mcp-tools', '');
  clearMcpServerBlock();
  const keyInput = input('settings-llm-api-key');
  if (keyInput) keyInput.value = '';
  const panelId = opts.panel || 'notes';
  switchSettingsTab('llm', 'engine');
  switchSettingsTab('mcp', panelId === 'mcp' ? (opts.tab || 'tickets') : 'tickets');
  const loadingWorkspace = showPanel(panelId);
  await loadSettingsSnapshot();
  await loadDisabledWorkspaces();
  await loadMcpChannelTools();
  requestAnimationFrame(() => paintMcpToolGroups());
  await loadMcpTicketView();
  await loadingWorkspace;
}

function wireSettingsDialog() {
  document.querySelectorAll('.settings-tab').forEach((tabBtn) => {
    tabBtn.addEventListener('click', () => {
      const panel = tabBtn.closest('.settings-panel');
      const panelId = panel?.id?.replace(/^settings-panel-/, '');
      const tab = (tabBtn as HTMLElement).dataset.tab;
      if (panelId && tab) switchSettingsTab(panelId, tab);
      if (panelId === 'mcp' && tab === 'tools') void loadMcpChannelTools();
    });
  });

  // Workspace nav items are painted by React and change with the registry: delegate.
  document.getElementById('settings-nav')?.addEventListener('click', (event) => {
    const navBtn = (event.target as HTMLElement | null)?.closest('.settings-nav-item') as HTMLElement | null;
    const panelId = navBtn?.dataset.panel;
    if (panelId) void showPanel(panelId);
  });

  btn('btn-settings-close')?.addEventListener('click', closeSettingsDialog);
  document.getElementById('settings-dialog')?.addEventListener('click', (e) => {
    if (e.target === document.getElementById('settings-dialog')) closeSettingsDialog();
  });
  document.getElementById('settings-theme')?.addEventListener('change', (e) => {
    void saveThemeSelection((e.target as HTMLSelectElement).value);
  });

  document.getElementById('settings-llm-engine')?.addEventListener('change', (e) => {
    void applyEngineCategorySelection((e.target as HTMLSelectElement).value, { clearCredential: true });
  });

  const keyInput = input('settings-llm-api-key');
  keyInput?.addEventListener('focus', () => clearSavedCredentialPlaceholder(keyInput));
  keyInput?.addEventListener('blur', () => restoreSavedCredentialPlaceholder(keyInput));

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
