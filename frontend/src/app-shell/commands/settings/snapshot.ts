import * as api from '../../../host/api.ts';
import type { SettingsConfig } from '../../state/types.ts';
import { setGithubUserUrl } from '../../../host/constants.ts';
import { state } from '../../../host/state.ts';
import { loadAssistantEnginePanel } from './engine.ts';
import {
  clearGithubUserUrlInferredLock,
  syncGithubUserUrlLockFromSparkRoot,
} from './github-user.ts';
import { renderSparkConnection, syncSparkConnectionAccess } from './spark-github.ts';
import { savedSnapshot, setResult, store } from '../../state/settings/store.ts';

export async function loadSettingsSnapshot() {
  try {
    const cfg = (await api.fetchConfig()) as SettingsConfig;

    const sparkInput = document.getElementById('settings-spark-root') as HTMLInputElement | null;
    const githubUserInput = document.getElementById('settings-github-user-url') as HTMLInputElement | null;
    const knowledgeInput = document.getElementById('knowledge-root-path') as HTMLInputElement | null;
    const wbRoot = cfg?.spark_root ?? '';
    const knowledgeRoot = cfg?.knowledge_root ?? '';
    const ghUrl = cfg?.github_user_url ?? '';
    if (wbRoot && sparkInput) {
      sparkInput.placeholder = wbRoot;
      sparkInput.value = wbRoot;
    }
    if (knowledgeInput) {
      knowledgeInput.value = knowledgeRoot || state.ui.knowledgeRoot || '';
      if (knowledgeRoot) {
        knowledgeInput.placeholder = knowledgeRoot;
        state.ui.knowledgeRoot = knowledgeRoot;
      }
    }
    clearGithubUserUrlInferredLock();
    if (githubUserInput) {
      githubUserInput.value = ghUrl;
    }
    setGithubUserUrl(ghUrl);
    savedSnapshot.sparkRoot = wbRoot;
    savedSnapshot.githubUserUrl = ghUrl;
    savedSnapshot.sparkGithubRepoUrl = cfg?.spark_github_repo_url ?? '';
    savedSnapshot.hasGithubToken = Boolean(cfg?.has_github_token);
    renderSparkConnection(savedSnapshot.sparkGithubRepoUrl);
    setResult('spark-connect-error', '');

    const hintEl = document.getElementById('settings-token-hint') as HTMLElement;
    hintEl.textContent = cfg?.has_github_token
      ? 'Sync token configured. Enter a new token to replace it.'
      : 'No Sync token configured.';

    loadAssistantEnginePanel(cfg ?? {});
    if (typeof cfg?.mcp_port === 'number' && cfg.mcp_port > 0) {
      store.mcpPort = cfg.mcp_port;
    }

    await syncGithubUserUrlLockFromSparkRoot();
    syncSparkConnectionAccess();
  } catch {
    const tokenHint = document.getElementById('settings-token-hint');
    if (tokenHint) tokenHint.textContent = 'Could not load settings; you can type and save.';
    const llmHint = document.getElementById('settings-llm-key-hint');
    if (llmHint) llmHint.textContent = 'Could not load settings; you can type and save.';
    loadAssistantEnginePanel({});
    clearGithubUserUrlInferredLock();
    savedSnapshot.githubUserUrl = '';
    savedSnapshot.sparkGithubRepoUrl = '';
    savedSnapshot.hasGithubToken = false;
    renderSparkConnection('');
    syncSparkConnectionAccess();
  }
}
