export type KnowledgeViewerState = {
  repo: string;
  path: string;
};

export function isKnowledgeMdPath(path: string) {
  return /\.md$/i.test(path.trim());
}

export function normalizeViewerState(raw: unknown): KnowledgeViewerState {
  const rec = raw && typeof raw === 'object' ? (raw as Record<string, unknown>) : {};
  const repo = typeof rec.repo === 'string' ? rec.repo.trim() : '';
  const path = typeof rec.path === 'string' ? rec.path.trim() : '';
  return { repo, path: isKnowledgeMdPath(path) ? path : '' };
}

export function knowledgeDocHash(repo: string, path = '') {
  const base = `#/knowledge/${encodeURIComponent(repo)}`;
  return path ? `${base}?path=${encodeURIComponent(path)}` : base;
}

export function resolveKnowledgeLanding(
  urlRepo: string,
  urlPath: string,
  repos: string[],
  remembered: KnowledgeViewerState,
): KnowledgeViewerState {
  if (urlRepo) {
    if (urlPath) return { repo: urlRepo, path: urlPath };
    if (remembered.repo === urlRepo) return { repo: urlRepo, path: remembered.path };
    return { repo: urlRepo, path: '' };
  }
  if (remembered.repo && repos.includes(remembered.repo)) {
    return { repo: remembered.repo, path: remembered.path };
  }
  return { repo: repos[0] || '', path: '' };
}
