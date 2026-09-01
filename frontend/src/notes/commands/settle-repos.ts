import * as api from '../../host/api.ts';
import type { SettleRepoOption } from '../state/settle.ts';

type TopicRec = { repo?: string; description?: string };
type TopicsFile = { topics?: TopicRec[] };

let topicsCache: TopicsFile | null = null;

export async function loadKnowledgeRepos(): Promise<SettleRepoOption[]> {
  if (!topicsCache) {
    topicsCache = (await api.fetchTopics()) as TopicsFile;
  }
  const repos: SettleRepoOption[] = [];
  for (const t of topicsCache.topics || []) {
    const fullName = typeof t.repo === 'string' ? t.repo.trim() : '';
    if (!fullName) continue;
    repos.push({
      fullName,
      description: typeof t.description === 'string' ? t.description : '',
    });
  }
  repos.sort((a, b) => a.fullName.localeCompare(b.fullName, 'en', { sensitivity: 'base' }));
  return repos;
}

export async function fetchSettleRepoDirs(repo: string) {
  return (await api.fetchRepoDirs(repo)) as { error?: string; dirs?: string[] };
}
