export const state = {
  index: {
    data: null,
    groupedByDate: [],
    titleCache: new Map(),
    diffStatus: new Map(),
    annotations: {},
    titleFetchCache: new Map()
  },
  ui: {
    activeDate: null,
    archiveRoot: ''
  },
  viewer: {
    entry: null,
    layer: 'raw',
    rawText: '',
    annotation: {},
    commentEditCtx: null
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
