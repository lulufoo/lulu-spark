import * as api from './api.js'

export const state = {
  index: {
    data: null,
    groupedByDate: [],
    filteredGroups: [],
    titleCache: new Map(),
    diffStatus: new Map(),
    annotations: {},
    titleFetchCache: new Map(),
    topicDescriptions: {},
    topicRepos: {}
  },
  ui: {
    activeDate: null,
    archiveRoot: '',
    kbRoot: '',
    activeTopic: null,
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
  }
}

export function mergeAnnotations(indexData, summary) {
  for (const entry of Object.values(indexData)) {
    const ann = summary[entry.common_path]
    entry.done = ann?.done || undefined
    entry.importance = ann?.importance || undefined
    entry.links = ann?.links || undefined
    entry._comment_counts = ann?.comment_counts || undefined
  }
}

export function buildPathToId(indexData) {
  const map = new Map()
  for (const [id, entry] of Object.entries(indexData)) map.set(entry.common_path, id)
  return map
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
