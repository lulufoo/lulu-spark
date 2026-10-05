import * as api from '../../../host/api.ts';
import type { SettingsConfig } from '../../state/types.ts';
import { state } from '../../../host/state.ts';
import { loadAssistantEnginePanel } from './engine.ts';
import { store } from '../../state/settings/store.ts';

export async function loadSettingsSnapshot() {
  try {
    const cfg = (await api.fetchConfig()) as SettingsConfig;

    const knowledgeInput = document.getElementById(
      'knowledge-root-path',
    ) as HTMLInputElement | null;
    const knowledgeRoot = cfg?.knowledge_root ?? '';
    if (knowledgeInput) {
      knowledgeInput.value = knowledgeRoot || state.ui.knowledgeRoot || '';
      if (knowledgeRoot) {
        knowledgeInput.placeholder = knowledgeRoot;
        state.ui.knowledgeRoot = knowledgeRoot;
      }
    }

    const notesInput = document.getElementById('notes-root-path') as HTMLInputElement | null;
    const notesRoot = cfg?.notes_root ?? '';
    if (notesInput) {
      notesInput.value = notesRoot;
      if (notesRoot) notesInput.placeholder = notesRoot;
    }

    loadAssistantEnginePanel(cfg ?? {});
    if (typeof cfg?.mcp_port === 'number' && cfg.mcp_port > 0) {
      store.mcpPort = cfg.mcp_port;
    }
  } catch {
    const llmHint = document.getElementById('settings-llm-key-hint');
    if (llmHint) llmHint.textContent = 'Could not load settings; you can type and save.';
    loadAssistantEnginePanel({});
  }
}
