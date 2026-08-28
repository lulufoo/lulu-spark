import { createModuleStore } from '../../shared/module-store.ts';

export type ConvertTab = 'base64' | 'qr';
export type ConvertState = { open: boolean; tab: ConvertTab };

export const convertStore = createModuleStore<ConvertState>({ open: false, tab: 'base64' });
