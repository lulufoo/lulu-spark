import { createModuleStore } from '../../shared/module-store.ts';

/** The document whose file name is being edited. `path` is that document's `raw_path`. */
export type FileRenameSnap = {
  id: string;
  project: string;
  path: string;
  name: string;
  saving: boolean;
  tick: number;
};

export const fileRenameStore = createModuleStore<FileRenameSnap>({
  id: '',
  project: '',
  path: '',
  name: '',
  saving: false,
  tick: 0,
});
