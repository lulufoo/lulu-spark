import { createModuleStore } from '../../shared/module-store.ts';

/** The directory-tree folder being dragged: its project-relative path and its tree node. */
export type FolderMoveSnap = {
  /** The dragged folder's path, relative to the project, such as `plans/q1`. */
  path: string;
  /** Node key of the folder row the drag started from, `<project id>/<sub>/<sub>`. */
  fromKey: string;
  /** Node key of the folder row highlighted as the drop target, '' when none. */
  over: string;
};

export const emptyFolderMove = (): FolderMoveSnap => ({ path: '', fromKey: '', over: '' });

export const folderMoveStore = createModuleStore<FolderMoveSnap>(emptyFolderMove());
