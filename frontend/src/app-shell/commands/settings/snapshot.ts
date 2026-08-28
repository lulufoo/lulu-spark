// @ts-nocheck — ported from JS; settings DOM wiring stays unchecked like checkJs:false.
import * as api from '../../../host/api.ts';
import { getKbHidePattern } from '../../../corpus/state/hide-pattern.ts';
import { setGithubUserUrl } from '../../../host/constants.ts';
import { state } from '../../../host/state.ts';
import { loadAssistantEnginePanel } from './engine.ts';
import {
  clearGithubUserUrlInferredLock,
  syncGithubUserUrlLockFromWorkbenchRoot,
} from './github-user.ts';
import { renderNotesConnection, syncNotesConnectionAccess } from './notes-github.ts';
import { savedSnapshot, setResult, store } from '../../state/settings/store.ts';

export function syncKbHidePatternInput() {
  const kbHideInput = document.getElementById('settings-kb-hide-pattern');
  if (kbHideInput) {
    kbHideInput.value = getKbHidePattern();
  }
}

export async function loadSettingsSnapshot() {
  try {
    const cfg = await api.fetchConfig();

    const archiveInput = document.getElementById('settings-archive-root');
    const githubUserInput = document.getElementById('settings-github-user-url');
    const corpusInput = document.getElementById('sediment-kb-corpus-path');
    const wbRoot = cfg?.workbench_knowledge_root ?? '';
    const corpusRoot = cfg?.knowledge_corpus_root ?? '';
    const ghUrl = cfg?.github_user_url ?? '';
    if (wbRoot) {
      archiveInput.placeholder = wbRoot;
      archiveInput.value = wbRoot;
    }
    if (corpusInput) {
      corpusInput.value = corpusRoot || state.ui.knowledgeCorpusRoot || '';
      if (corpusRoot) {
        corpusInput.placeholder = corpusRoot;
        state.ui.knowledgeCorpusRoot = corpusRoot;
      }
    }
    clearGithubUserUrlInferredLock();
    if (githubUserInput) {
      githubUserInput.value = ghUrl;
    }
    setGithubUserUrl(ghUrl);
    savedSnapshot.workbenchKnowledgeRoot = wbRoot;
    savedSnapshot.githubUserUrl = ghUrl;
    savedSnapshot.workbenchGithubRepoUrl = cfg?.workbench_github_repo_url ?? '';
    savedSnapshot.hasGithubToken = Boolean(cfg?.has_github_token);
    renderNotesConnection(savedSnapshot.workbenchGithubRepoUrl);
    setResult('notes-connect-error', '');

    const hintEl = document.getElementById('settings-token-hint');
    hintEl.textContent = cfg?.has_github_token
      ? 'Sync token configured. Enter a new token to replace it.'
      : 'No Sync token configured.';

    loadAssistantEnginePanel(cfg ?? {});
    if (typeof cfg?.mcp_port === 'number' && cfg.mcp_port > 0) {
      store.mcpPort = cfg.mcp_port;
    }

    await syncGithubUserUrlLockFromWorkbenchRoot();
    syncKbHidePatternInput();
    syncNotesConnectionAccess();
  } catch {
    document.getElementById('settings-token-hint').textContent =
      'Could not load settings; you can type and save.';
    document.getElementById('settings-llm-key-hint').textContent =
      'Could not load settings; you can type and save.';
    loadAssistantEnginePanel({});
    clearGithubUserUrlInferredLock();
    syncKbHidePatternInput();
    savedSnapshot.githubUserUrl = '';
    savedSnapshot.workbenchGithubRepoUrl = '';
    savedSnapshot.hasGithubToken = false;
    renderNotesConnection('');
    syncNotesConnectionAccess();
  }
}
