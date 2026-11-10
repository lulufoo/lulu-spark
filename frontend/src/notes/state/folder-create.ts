import { createModuleStore } from '../../shared/module-store.ts';

export type FolderMenu = { parent: string; x: number; y: number };

/** The folder-name row, plus the right-click menu that opens it. */
export type FolderCreateSnap = {
  /** Project the folder will belong to. Empty while the row is hidden. */
  project: string;
  /** Folder it is created under. `/` is the project root. */
  parent: string;
  name: string;
  saving: boolean;
  tick: number;
  menu: FolderMenu | null;
};

export const folderCreateStore = createModuleStore<FolderCreateSnap>({
  project: '',
  parent: '/',
  name: '',
  saving: false,
  tick: 0,
  menu: null,
});
