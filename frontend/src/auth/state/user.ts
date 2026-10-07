import { createModuleStore } from '../../shared/module-store.ts';
import type { AuthUserView } from '../oauth.ts';

export const authUserStore = createModuleStore<AuthUserView | null>(null);
