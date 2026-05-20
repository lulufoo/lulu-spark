const DEFAULT_CORPUS_GITHUB = 'https://github.com/lulufoo/lulu-workbench-knowledge/blob/main'

let corpusGithub = DEFAULT_CORPUS_GITHUB

/** @deprecated Use getCorpusGithub() for corpus file links */
export const REPO = DEFAULT_CORPUS_GITHUB

export function setCorpusGithub(url) {
  const base = (url || DEFAULT_CORPUS_GITHUB).replace(/\/$/, '')
  corpusGithub = base || DEFAULT_CORPUS_GITHUB
}

export function getCorpusGithub() {
  return corpusGithub
}

export const LAYERS = ['raw', 'distilled', 'digest', 'trace']
export const IMPORTANCE_CYCLE = [undefined, 'high', 'medium', 'low']
