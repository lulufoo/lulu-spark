import * as api from '../../host/api.ts';
import {
  isKnowledgeMdPath,
  knowledgeDocHash,
  knowledgeTopicHash,
  normalizeViewerState,
  type KnowledgeViewerState,
} from '../state/viewer-state.ts';

export async function loadKbViewerState(): Promise<KnowledgeViewerState> {
  try {
    return normalizeViewerState(await api.fetchKbViewerState());
  } catch {
    return { id: '' };
  }
}

export async function saveKbViewerState(id: string): Promise<void> {
  try {
    await api.saveKbViewerState(id);
  } catch {
    /* cache write is best-effort */
  }
}

export async function knowledgeLandingHash(topics: string[]): Promise<string> {
  const remembered = await loadKbViewerState();
  if (remembered.id) {
    try {
      const doc = await api.resolveKnowledgeForOpen(remembered.id);
      if (doc.topic && topics.includes(doc.topic)) {
        return knowledgeDocHash(doc.topic, remembered.id);
      }
    } catch {
      /* fall through to the first topic */
    }
  }
  return topics[0] ? knowledgeTopicHash(topics[0]) : '';
}

export async function applyKbViewerRestore({
  topic,
  docId,
  ensureVisible,
  hasFile,
  openDoc,
  syncHash,
}: {
  topic: string;
  docId: string;
  ensureVisible: (path: string) => Promise<void>;
  hasFile: (path: string) => boolean;
  openDoc: (id: string) => Promise<void>;
  syncHash: (id: string) => void;
}): Promise<void> {
  if (docId) {
    await openDoc(docId);
    await saveKbViewerState(docId);
    return;
  }
  const remembered = await loadKbViewerState();
  if (!remembered.id) {
    await saveKbViewerState('');
    return;
  }
  try {
    const doc = await api.resolveKnowledgeForOpen(remembered.id);
    if (doc.topic !== topic) {
      await saveKbViewerState('');
      return;
    }
    if (doc.path) await ensureVisible(doc.path);
    if (doc.path && !hasFile(doc.path)) {
      await saveKbViewerState('');
      return;
    }
    await openDoc(remembered.id);
    syncHash(remembered.id);
    await saveKbViewerState(remembered.id);
  } catch {
    await saveKbViewerState('');
  }
}

export { isKnowledgeMdPath };
