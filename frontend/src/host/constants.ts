let githubUserUrl = ''

/** @deprecated */
export const REPO = ''

export function setGithubUserUrl(url?: string | null) {
  githubUserUrl = (url || '').replace(/\/$/, '')
}

export function getGithubUserUrl(): string {
  return githubUserUrl
}

/** `https://github.com/{owner}` + spark clone dir name → blob base for file links. */
export function sparkGithubBlobBase(
  githubUserUrlArg?: string | null,
  sparkRoot?: string | null,
): string {
  const trimmed = (githubUserUrlArg || '').trim().replace(/\/$/, '')
  if (!trimmed) return ''
  const parts = (sparkRoot || '').split(/[/\\]/).filter(Boolean)
  const repo = parts.length ? parts[parts.length - 1] : 'lulu-workbench-knowledge'
  return `${trimmed}/${repo}/blob/main`
}

export const NOTES_DIR = 'notes'

export function notesFileRelPath(layer: string, activePath: string): string {
  return `${NOTES_DIR}/${layer}/${activePath}`
}

export const TAG_VALUE_MAX_LEN = 64
export const TAG_SUGGEST_MIN_SCORE = 0.6

export const LAYERS = ['raw', 'digest']
export const IMPORTANCE_CYCLE = [undefined, 'high', 'medium', 'low']
