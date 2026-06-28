import * as api from './api.js';
import { escHtml } from './utils.js';

const DEFAULT_POLL_ATTEMPTS = 120;
const DEFAULT_POLL_INTERVAL_MS = 2000;

function defaultSleep(ms) {
  return new Promise(resolve => setTimeout(resolve, ms));
}

async function waitForReindexToSettle(statusFn, { sleep, pollAttempts, pollIntervalMs }) {
  for (let i = 0; i < pollAttempts; i++) {
    await sleep(pollIntervalMs);
    const status = await statusFn();
    if (status?.status !== 'running') return status;
  }
  return null;
}

export async function runSedimentKbSync(repo, {
  apiClient = api,
  sleep = defaultSleep,
  pollAttempts = DEFAULT_POLL_ATTEMPTS,
  pollIntervalMs = DEFAULT_POLL_INTERVAL_MS,
} = {}) {
  if (!repo) throw new Error('请选择一个具体仓库后再同步。');
  await apiClient.reindexKbRepo(repo);
  return waitForReindexToSettle(() => apiClient.getReindexStatus(), {
    sleep,
    pollAttempts,
    pollIntervalMs,
  });
}

export async function runWorkbenchKnowledgeSync({
  apiClient = api,
  sleep = defaultSleep,
  pollAttempts = DEFAULT_POLL_ATTEMPTS,
  pollIntervalMs = DEFAULT_POLL_INTERVAL_MS,
} = {}) {
  await apiClient.syncKnowledgeCorpus();
  await apiClient.reindexWorkbench();
  return waitForReindexToSettle(() => apiClient.getReindexWorkbenchStatus(), {
    sleep,
    pollAttempts,
    pollIntervalMs,
  });
}

export function renderKnowledgeSyncControls({
  container,
  kind,
  repo = '',
  localExists = true,
  onSync,
} = {}) {
  if (!container) return;
  const isSediment = kind === 'sediment';
  const isWorkbench = kind === 'workbench';
  const canSync = isWorkbench || (Boolean(repo) && localExists === true);
  const message = isWorkbench
    ? '同步 workbench 知识库'
    : !repo
      ? '请选择一个具体仓库后再同步。'
      : localExists === false
        ? '该仓库未克隆，不能重建本地索引。'
        : `同步 ${repo}`;
  const buttonLabel = isSediment ? 'SYNC' : '同步';

  container.innerHTML = `<div class="knowledge-sync-controls" data-knowledge-sync-controls>
    <span class="knowledge-sync-status" data-knowledge-sync-status>${escHtml(message)}</span>
    <button type="button" data-knowledge-sync-button${canSync ? '' : ' disabled'}>${buttonLabel}</button>
  </div>`;

  const button = container.querySelector('[data-knowledge-sync-button]');
  if (!button || !canSync || typeof onSync !== 'function') return;
  button.addEventListener('click', () => {
    onSync(isSediment ? repo : undefined);
  });
}
