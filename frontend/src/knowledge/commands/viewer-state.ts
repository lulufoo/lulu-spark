import * as api from '../../host/api.ts';
import {
  isKnowledgeMdPath,
  knowledgeDocHash,
  normalizeViewerState,
  resolveKnowledgeLanding,
  type KnowledgeViewerState,
} from '../state/viewer-state.ts';

export async function loadKbViewerState(): Promise<KnowledgeViewerState> {
  try {
    return normalizeViewerState(await api.fetchKbViewerState());
  } catch {
    return { repo: '', path: '' };
  }
}

export async function saveKbViewerState(repo: string, path: string): Promise<void> {
  try {
    await api.saveKbViewerState(repo, path);
  } catch {
    /* cache write is best-effort */
  }
}

export async function knowledgeLandingHash(repos: string[]): Promise<string> {
  const remembered = await loadKbViewerState();
  const landing = resolveKnowledgeLanding('', '', repos, remembered);
  return landing.repo ? knowledgeDocHash(landing.repo, landing.path) : '';
}

export async function applyKbViewerRestore({
  repo,
  selectedPath,
  ensureVisible,
  hasFile,
  openPath,
  syncHash,
}: {
  repo: string;
  selectedPath: string;
  ensureVisible: (path: string) => Promise<void>;
  hasFile: (path: string) => boolean;
  openPath: (path: string) => Promise<void>;
  syncHash: (path: string) => void;
}): Promise<void> {
  if (selectedPath) {
    await openPath(selectedPath);
    if (isKnowledgeMdPath(selectedPath)) await saveKbViewerState(repo, selectedPath);
    return;
  }
  const remembered = await loadKbViewerState();
  if (remembered.repo === repo && remembered.path) {
    await ensureVisible(remembered.path);
    if (hasFile(remembered.path)) {
      await openPath(remembered.path);
      syncHash(remembered.path);
      await saveKbViewerState(repo, remembered.path);
      return;
    }
  }
  await saveKbViewerState(repo, '');
}
