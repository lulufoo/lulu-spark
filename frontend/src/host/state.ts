// @ts-nocheck — index / viewer shapes stay unchecked like checkJs:false.
import { useSyncExternalStore } from 'react'
import * as api from './api.ts'

let stateVersion = 0
const stateListeners = new Set()

export function subscribeState(listener) {
  stateListeners.add(listener)
  return () => {
    stateListeners.delete(listener)
  }
}

export function notifyState() {
  stateVersion += 1
  stateListeners.forEach((fn) => fn())
}

export function getStateVersion() {
  return stateVersion
}

/** Pages read `state` after this hook so a notify re-renders them. */
export function useHostState() {
  useSyncExternalStore(subscribeState, getStateVersion, getStateVersion)
  return state
}

export const state = {
  index: {
    data: null,
    groupedByDate: [],
    filteredGroups: [],
    titleCache: new Map(),
    diffStatus: new Map(),
    annotations: {},
    tagsRegistry: { keys: {} },
    titleFetchCache: new Map(),
    topicDescriptions: {},
    topicRepos: {}
  },
  ui: {
    activeDate: null,
    workbenchKnowledgeRoot: '',
    knowledgeCorpusRoot: '',
    githubUserUrl: '',
    activeTopic: null,
    activeTagKey: null,
  },
  viewer: {
    entry: null,
    layer: 'raw',
    lang: null,
    rawText: '',
    annotation: {},
    commentEditCtx: null,
    scrollCache: {},  // key: "entryId:layer" → scrollTop
    isKb: false,
    kbRepo: null,
    kbPath: null,
    /** @type {{ tempId: string, status: 'creating' | 'saving' } | null} */
    createSession: null,
  }
}

export function mergeAnnotations(indexData, summary) {
  for (const entry of Object.values(indexData)) {
    const ann = summary[entry.common_path]
    entry.done = ann?.done || undefined
    entry.importance = ann?.importance || undefined
    entry.links = ann?.links || undefined
    entry._comment_counts = ann?.comment_counts || undefined
    if (ann?.tags) {
      entry.tags = ann.tags
      entry.tag_keys = ann.tag_keys
    } else {
      delete entry.tags
      delete entry.tag_keys
    }
  }
}

export function getEntryId(entry) {
  return entry._id || null
}

export async function loadDiffStatus() {
  try {
    const data = await api.fetchDiffStatus()
    if (!data) return
    state.index.diffStatus.clear()
    for (const p of (data.modified || [])) state.index.diffStatus.set(p, 'modified')
    for (const p of (data.conflicted || [])) state.index.diffStatus.set(p, 'conflict')
  } catch { /* non-critical */ }
}
