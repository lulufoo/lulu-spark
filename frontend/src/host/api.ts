export {
  assertReadPayload,
  assertWritePayload,
  createChannel,
  invoke,
} from './api/transport.ts';

export {
  getMessageChannelUnread,
  markMessageChannelRead,
} from './api/message-center.ts';

export { showOsNotification } from './api/os-notification.ts';

export { resolveNoteForOpen } from './api/note-open.ts';
export type { ResolvedNote } from './api/note-open.ts';

export { resolveKnowledgeForOpen } from './api/knowledge-open.ts';
export type { ResolvedKnowledgeDoc } from './api/knowledge-open.ts';

export { getPackageSnapshot } from './api/package-snapshot.ts';
export type { PackageSnapshot } from './api/package-snapshot.ts';

export {
  createNotesCategory,
  deleteNotesCategory,
  fetchNotesCategories,
  updateNotesCategory,
} from './api/notes-categories.ts';

export {
  createNote,
  clearNoteDraft,
  deleteEntry,
  fetchAnnotation,
  fetchAnnotationsSummary,
  fetchConfig,
  fetchNotesAssetAsBlobUrl,
  fetchDocHighlights,
  fetchFileContent,
  fetchIndex,
  fetchLinkTitle,
  fetchTagsRegistry,
  fetchTopics,
  getDraft,
  getNoteDraft,
  moveToProject,
  reorderComments,
  saveDraft,
  saveFile,
  saveNoteDraft,
  setConfig,
  setDone,
  setImportance,
  tagAttach,
  tagDetach,
  tagUpdateValue,
  updateComments,
  updateDocHighlights,
  updateHighlight,
  updateLinks,
} from './api/spark.ts';

export {
  addSedimentKbCategory,
  addSedimentKbRepo,
  fetchKbAnnotation,
  addKbHidePattern,
  fetchKbDocCount,
  fetchKbHidePatterns,
  fetchKbViewerState,
  fetchKbAssetAsBlobUrl,
  fetchKbFileContent,
  fetchKbList,
  fetchSedimentKbCategories,
  fetchSedimentKbRepos,
  getReindexAllStatus,
  getReindexStatus,
  reindexAll,
  reindexKbRepo,
  removeKbHidePattern,
  createKbEntry,
  deleteKbEntry,
  renameKbEntry,
  removeSedimentKbCategory,
  removeSedimentKbRepo,
  renameSedimentKbCategory,
  reorderKbComments,
  saveKbFile,
  saveKbViewerState,
  searchKnowledge,
  searchSpark,
  updateKbHidePattern,
  updateKbComment,
  updateKbHighlight,
  updateKbLinks,
  updateSedimentKbRepoCategory,
} from './api/knowledge.ts';
