import { TAG_SUGGEST_MIN_SCORE } from '../../host/constants.ts';

function levenshtein(a: string, b: string): number {
  const m = a.length;
  const n = b.length;
  if (m === 0) return n;
  if (n === 0) return m;
  const dp = Array.from({ length: m + 1 }, () => new Array<number>(n + 1).fill(0));
  for (let i = 0; i <= m; i++) dp[i][0] = i;
  for (let j = 0; j <= n; j++) dp[0][j] = j;
  for (let i = 1; i <= m; i++) {
    for (let j = 1; j <= n; j++) {
      const cost = a[i - 1] === b[j - 1] ? 0 : 1;
      dp[i][j] = Math.min(dp[i - 1][j] + 1, dp[i][j - 1] + 1, dp[i - 1][j - 1] + cost);
    }
  }
  return dp[m][n];
}

function scoreMatch(query: string, value: string, key: string): number {
  const q = query.toLowerCase();
  const v = (value || '').toLowerCase();
  const k = (key || '').toLowerCase();
  if (v.includes(q) || k.includes(q)) return 1;
  const dist = levenshtein(q, v);
  const maxLen = Math.max(q.length, v.length, 1);
  return 1 - dist / maxLen;
}

type TagMeta = { value?: string };
type TagRegistry = { keys?: Record<string, TagMeta> };

export function suggestTags(
  query: string | null | undefined,
  registry: TagRegistry | null | undefined,
): Array<{ key: string; value: string; score: number }> {
  const q = (query || '').trim();
  if (!q) return [];
  const keys = registry?.keys || {};
  const results: Array<{ key: string; value: string; score: number }> = [];
  for (const [key, meta] of Object.entries(keys)) {
    const value = meta?.value || '';
    const score = scoreMatch(q, value, key);
    if (score >= TAG_SUGGEST_MIN_SCORE) results.push({ key, value, score });
  }
  return results.sort((a, b) => b.score - a.score).slice(0, 12);
}
