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


export { getPackageSnapshot } from './api/package-snapshot.ts';
export type { PackageSnapshot } from './api/package-snapshot.ts';

export {
  createProject,
  createWorkspace,
  deleteProject,
  deleteWorkspace,
  fetchProjects,
  fetchWorkspaces,
  renameWorkspace,
  setWorkspaceViewMode,
  updateProject,
} from './api/workspaces.ts';
export type { ViewMode } from './api/workspaces.ts';

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

export { getReindexAllStatus, reindexAll, searchSpark } from './api/search.ts';
