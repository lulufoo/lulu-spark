import * as api from '../../../host/api.ts';
import type { SettingsConfig } from '../../state/types.ts';
import { loadAssistantEnginePanel } from './engine.ts';
import { loadThemePanel } from './theme.ts';
import { store } from '../../state/settings/store.ts';

export async function loadSettingsSnapshot() {
  try {
    const cfg = (await api.fetchConfig()) as SettingsConfig;

    const rootInput = document.getElementById('workspaces-root-path') as HTMLInputElement | null;
    const workspacesRoot = cfg?.workspaces_root ?? '';
    if (rootInput) {
      rootInput.value = workspacesRoot;
      if (workspacesRoot) rootInput.placeholder = workspacesRoot;
    }

    loadAssistantEnginePanel(cfg ?? {});
    loadThemePanel(cfg?.theme);
    if (typeof cfg?.mcp_port === 'number' && cfg.mcp_port > 0) {
      store.mcpPort = cfg.mcp_port;
    }
  } catch {
    const llmHint = document.getElementById('settings-llm-key-hint');
    if (llmHint) llmHint.textContent = 'Could not load settings; you can type and save.';
    loadAssistantEnginePanel({});
    loadThemePanel();
  }
}
