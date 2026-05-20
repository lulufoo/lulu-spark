let githubUserUrl = ''

/** @deprecated */
export const REPO = ''

export function setGithubUserUrl(url) {
  githubUserUrl = (url || '').replace(/\/$/, '')
}

export function getGithubUserUrl() {
  return githubUserUrl
}

/** `https://github.com/{owner}` + workbench clone dir name → blob base for file links. */
export function workbenchGithubBlobBase(githubUserUrlArg, workbenchKnowledgeRoot) {
  const trimmed = (githubUserUrlArg || '').trim().replace(/\/$/, '')
  if (!trimmed) return ''
  const parts = (workbenchKnowledgeRoot || '').split(/[/\\]/).filter(Boolean)
  const repo = parts.length ? parts[parts.length - 1] : 'lulu-workbench-knowledge'
  return `${trimmed}/${repo}/blob/main`
}

export const LAYERS = ['raw', 'distilled', 'digest', 'trace']
export const IMPORTANCE_CYCLE = [undefined, 'high', 'medium', 'low']
