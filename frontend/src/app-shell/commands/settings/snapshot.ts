import * as api from '../../../host/api.ts';
import type { SettingsConfig } from '../../state/types.ts';
import { loadAssistantEnginePanel } from './engine.ts';
import { loadThemePanel } from './theme.ts';
import { store } from '../../state/settings/store.ts';
import { patchWorkspacesPanel } from '../../state/settings/workspaces-panel.ts';

export async function loadSettingsSnapshot() {
  try {
    const cfg = (await api.fetchConfig()) as SettingsConfig;

    patchWorkspacesPanel({ root: cfg?.workspaces_root ?? '' });

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
