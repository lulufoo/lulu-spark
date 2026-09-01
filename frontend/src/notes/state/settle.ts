import { createModuleStore } from '../../shared/module-store.ts';

export type SettleResultKind = '' | 'ok' | 'err';

export type SettleRepoOption = {
  fullName: string;
  description: string;
};

export type SettleView = {
  repo: string;
  repos: SettleRepoOption[];
  slug: string;
  content: string;
  filenameTs: string;
  dirs: string[];
  dirsLoading: boolean;
  dirsError: string;
  themeSelect: string;
  themeInput: string;
  showThemeInput: boolean;
  fileWarnPath: string;
  resultKind: SettleResultKind;
  resultText: string;
  resultUrl: string;
  resultWarns: string[];
  submitLabel: string;
  submitDisabled: boolean;
};

export function emptySettleView(): SettleView {
  return {
    repo: '',
    repos: [],
    slug: '',
    content: '',
    filenameTs: '',
    dirs: [],
    dirsLoading: false,
    dirsError: '',
    themeSelect: '',
    themeInput: '',
    showThemeInput: false,
    fileWarnPath: '',
    resultKind: '',
    resultText: '',
    resultUrl: '',
    resultWarns: [],
    submitLabel: 'Push',
    submitDisabled: false,
  };
}

export const settleViewStore = createModuleStore<SettleView>(emptySettleView());

export function patchSettle(partial: Partial<SettleView>) {
  settleViewStore.set({ ...settleViewStore.getSnapshot(), ...partial });
}
