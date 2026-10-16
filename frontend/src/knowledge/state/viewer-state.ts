export type KnowledgeViewerState = {
  id: string;
};

export function isKnowledgeMdPath(path: string) {
  return /\.md$/i.test(path.trim());
}

export function normalizeViewerState(raw: unknown): KnowledgeViewerState {
  const rec = raw && typeof raw === 'object' ? (raw as Record<string, unknown>) : {};
  const id = typeof rec.id === 'string' ? rec.id.trim() : '';
  return { id };
}

export function knowledgeTopicHash(topic: string) {
  return `#/knowledge/${encodeURIComponent(topic)}`;
}

export function knowledgeDocHash(topic: string, id = '') {
  const base = knowledgeTopicHash(topic);
  return id ? `${base}?id=${encodeURIComponent(id)}` : base;
}

export function resolveKnowledgeLanding(
  urlTopic: string,
  urlId: string,
  topics: string[],
  remembered: KnowledgeViewerState,
  rememberedTopic = '',
): { topic: string; id: string } {
  if (urlTopic) {
    if (urlId) return { topic: urlTopic, id: urlId };
    if (rememberedTopic === urlTopic) return { topic: urlTopic, id: remembered.id };
    return { topic: urlTopic, id: '' };
  }
  if (rememberedTopic && topics.includes(rememberedTopic)) {
    return { topic: rememberedTopic, id: remembered.id };
  }
  return { topic: topics[0] || '', id: rememberedTopic ? '' : remembered.id };
}
