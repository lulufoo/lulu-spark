import { useSyncExternalStore } from 'react'
import type { HostIndexAnnotation, HostNoteEntry, HostState } from './snapshot-types.ts'

export type {
  HostCreateSession,
  HostIndex,
  HostIndexAnnotation,
  HostNoteEntry,
  HostNoteGroup,
  HostNoteTag,
  HostOutletMode,
  HostState,
  HostTagMeta,
  HostTagsRegistry,
  HostUi,
  HostViewer,
  HostViewerAnnotation,
} from './snapshot-types.ts'

let stateVersion = 0
const stateListeners = new Set<() => void>()

export function subscribeState(listener: () => void) {
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

export const state: HostState = {
  index: {
    data: null,
    groupedByDate: [],
    filteredGroups: [],
    titleCache: new Map(),
    annotations: {},
    tagsRegistry: { keys: {} },
    titleFetchCache: new Map(),
    topicDescriptions: {},
    topicTitles: {},
    topicRepos: {},
  },
  ui: {
    activeDate: null,
    sparkRoot: '',
    knowledgeRoot: '',
    activeTopic: null,
    activeTagKey: null,
    loadError: null,
  },
  viewer: {
    entry: null,
    layer: 'raw',
    lang: null,
    rawText: '',
    annotation: {},
    commentEditCtx: null,
    scrollCache: {},
    isKb: false,
    kbRepo: null,
    kbPath: null,
    createSession: null,
    outletMode: '',
    outletMessage: '',
    fileSize: '',
    loadError: '',
    loading: false,
    editing: false,
    saving: false,
    bodyPaintKey: 0,
    panelTitle: '',
  },
}

export function mergeAnnotations(
  indexData: Record<string, HostNoteEntry>,
  summary: Record<string, HostIndexAnnotation>,
) {
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

export function getEntryId(entry: HostNoteEntry | null | undefined) {
  return entry?._id || null
}
