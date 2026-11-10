import { createModuleStore } from '../../shared/module-store.ts';

/** The folder whose name is being edited. `path` is relative to the project. */
export type FolderRenameSnap = {
  project: string;
  path: string;
  name: string;
  saving: boolean;
  tick: number;
};

export const folderRenameStore = createModuleStore<FolderRenameSnap>({
  project: '',
  path: '',
  name: '',
  saving: false,
  tick: 0,
});
