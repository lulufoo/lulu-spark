import { createModuleStore } from '../../shared/module-store.ts';

export type AboutState = {
  open: boolean;
  product_name: string;
  version: string;
};

export const aboutStore = createModuleStore<AboutState>({
  open: false,
  product_name: '',
  version: '',
});
