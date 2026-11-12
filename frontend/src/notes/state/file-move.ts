import { createModuleStore } from '../../shared/module-store.ts';

/** The directory-tree document being dragged: its id and the tree node it currently sits in. */
export type FileMoveSnap = {
  id: string;
  /** The dragged document's node key, `<project id>/<sub>/<sub>`. */
  fromKey: string;
  /** Node key of the folder row highlighted as the drop target, '' when none. */
  over: string;
};

export const emptyFileMove = (): FileMoveSnap => ({ id: '', fromKey: '', over: '' });

export const fileMoveStore = createModuleStore<FileMoveSnap>(emptyFileMove());
