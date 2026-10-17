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

export { resolveDocForOpen } from './api/doc-open.ts';
export type { ResolvedNote } from './api/doc-open.ts';


export { getPackageSnapshot } from './api/package-snapshot.ts';
export type { PackageSnapshot } from './api/package-snapshot.ts';

export {
  addHidePattern,
  createProject,
  createWorkspace,
  disableProject,
  disableWorkspace,
  enableProject,
  enableWorkspace,
  fetchDisabledProjects,
  fetchDisabledWorkspaces,
  fetchHidePatterns,
  fetchProjects,
  fetchViewerState,
  fetchWorkspaces,
  removeHidePattern,
  renameWorkspace,
  saveViewerState,
  setWorkspaceIcon,
  setWorkspaceViewMode,
  updateHidePattern,
  updateProject,
} from './api/workspaces.ts';
export type { HidePatternRow, ViewMode } from './api/workspaces.ts';

export {
  createNote,
  clearDocDraft,
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
  getDocDraft,
  moveToProject,
  reorderComments,
  saveDraft,
  saveFile,
  saveDocDraft,
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
