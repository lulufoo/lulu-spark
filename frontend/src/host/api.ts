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
export { deleteBoardPat, fetchBoardPatStatus, setBoardPat } from './api/board-pat.ts';
export type { PackageSnapshot } from './api/package-snapshot.ts';

export {
  addHidePattern,
  createProject,
  createWorkspace,
  deleteProject,
  deleteWorkspace,
  cancelMoveProject,
  moveProject,
  disableWorkspace,
  enableWorkspace,
  fetchDisabledWorkspaces,
  fetchHidePatterns,
  fetchProjects,
  fetchViewerState,
  fetchWorkspaces,
  removeHidePattern,
  renameWorkspace,
  reorderProjects,
  reorderWorkspaces,
  saveViewerState,
  setWorkspaceDescription,
  setWorkspaceIcon,
  setWorkspaceViewMode,
  updateHidePattern,
  updateProject,
} from './api/workspaces.ts';
export type { DeleteProjectResult, DeleteWorkspaceResult, HidePatternRow, MoveProjectResult, ViewMode, ViewerProjectState, ViewerState } from './api/workspaces.ts';

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
  setDocSourceType,
  setImportance,
  syncDocuments,
  cancelSyncDocuments,
  checkSyncDocuments,
  getSyncProgress,
  tagAttach,
  tagDetach,
  tagUpdateValue,
  updateComments,
  updateDocHighlights,
  updateHighlight,
  updateLinks,
} from './api/spark.ts';
export type {
  CancelSyncResult,
  CheckSyncResult,
  SyncProgressEntry,
  SyncProgressResult,
  SyncDocumentIssue,
  SyncDocumentsResult,
} from './api/spark.ts';

export { getReindexAllStatus, reindexAll, searchChats, searchSpark } from './api/search.ts';
export type { ChatSearchHit } from './api/search.ts';

export { fetchDocCategories, saveDocCategories } from './api/doc-categories.ts';
export type { DocCategory } from './api/doc-categories.ts';
