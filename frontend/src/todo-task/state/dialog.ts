import { createModuleStore } from '../../shared/module-store.ts';

export type TodoTaskDialogType =
  | 'create-master'
  | 'create-category'
  | 'add-sub'
  | 'delete-master'
  | 'delete-sub';

export type DialogView = {
  type: TodoTaskDialogType | null;
  title: string;
  primaryLabel: string;
  danger: boolean;
  error: string;
  loading: boolean;
  payload: Record<string, unknown>;
  titleField: string;
  nameField: string;
  subTitleField: string;
  subRows: string[];
};

export function emptyDialogView(): DialogView {
  return {
    type: null,
    title: '',
    primaryLabel: 'OK',
    danger: false,
    error: '',
    loading: false,
    payload: {},
    titleField: '',
    nameField: '',
    subTitleField: '',
    subRows: [''],
  };
}

export const openStore = createModuleStore(false);
export const viewStore = createModuleStore<DialogView>(emptyDialogView());

export function patchView(partial: Partial<DialogView>) {
  viewStore.set({ ...viewStore.getSnapshot(), ...partial });
}
