import { createModuleStore } from '../../shared/module-store.ts';

/** The directory-tree row that asks for a markdown file name before a document exists. */
export type DirCreateSnap = {
  /** Project the file will belong to. Empty while the row is hidden. */
  project: string;
  /** Folder under that project. `/` is the project root. */
  folder: string;
  name: string;
  saving: boolean;
  /** Bumped each time "+" opens the row, so the name field takes focus again. */
  tick: number;
};

export const dirCreateStore = createModuleStore<DirCreateSnap>({
  project: '',
  folder: '/',
  name: '',
  saving: false,
  tick: 0,
});
